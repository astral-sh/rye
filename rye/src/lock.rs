use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::{fmt, fs};

use anyhow::{anyhow, bail, Context, Error};
use minijinja::render;
use once_cell::sync::Lazy;
use pep508_rs::Requirement;
use regex::Regex;
use serde::Serialize;
use tempfile::NamedTempFile;
use url::Url;

use crate::piptools::get_pip_compile;
use crate::pyproject::{
    normalize_package_name, DependencyKind, ExpandedSources, PyProject, Workspace,
};
use crate::sources::PythonVersion;
use crate::utils::{get_venv_python_bin, set_proxy_variables, CommandOutput};

static FILE_EDITABLE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^-e (file://.*?)\s*$").unwrap());
static REQUIREMENTS_HEADER: &str = r#"# generated by rye
# use `rye lock` or `rye sync` to update this lockfile
#
# last locked with the following flags:
#   pre: {{ lock_options.pre }}
#   features: {{ lock_options.features }}
#   all-features: {{ lock_options.all_features }}
#   with-sources: {{ lock_options.with_sources }}

"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LockMode {
    Production,
    Dev,
}

impl fmt::Display for LockMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LockMode::Production => "production",
                LockMode::Dev => "dev",
            }
        )
    }
}

/// Controls how locking should work.
#[derive(Debug, Clone, Default, Serialize)]
pub struct LockOptions {
    /// Instruct all packages to update.
    pub update_all: bool,
    /// Update specific packages.
    pub update: Vec<String>,
    /// Pick pre-release versions.
    pub pre: bool,
    /// A list of features (extras) to enable when locking
    pub features: Vec<String>,
    /// Enable all features in the workspace.
    pub all_features: bool,
    /// Should locking happen with sources?
    pub with_sources: bool,
}

/// Creates lockfiles for all projects in the workspace.
pub fn update_workspace_lockfile(
    py_ver: &PythonVersion,
    workspace: &Arc<Workspace>,
    lock_mode: LockMode,
    lockfile: &Path,
    output: CommandOutput,
    sources: &ExpandedSources,
    lock_options: &LockOptions,
) -> Result<(), Error> {
    if output != CommandOutput::Quiet {
        echo!("Generating {} lockfile: {}", lock_mode, lockfile.display());
    }

    let features_by_project = collect_workspace_features(lock_options);
    let mut req_file = NamedTempFile::new()?;
    let mut local_req_file = NamedTempFile::new()?;

    let mut local_projects = HashMap::new();
    let mut projects = Vec::new();
    for pyproject_result in workspace.iter_projects() {
        let pyproject = pyproject_result?;
        let rel_url = make_relative_url(&pyproject.root_path(), &workspace.path())?;
        let applicable_extras = format_project_extras(features_by_project.as_ref(), &pyproject)?;

        // virtual packages are not installed
        if !pyproject.is_virtual() {
            writeln!(local_req_file, "-e {}{}", rel_url, applicable_extras)?;
        }

        local_projects.insert(pyproject.normalized_name()?, rel_url);
        projects.push(pyproject);
    }

    for pyproject in &projects {
        dump_dependencies(
            pyproject,
            &local_projects,
            req_file.as_file_mut(),
            DependencyKind::Normal,
        )?;
        if lock_mode == LockMode::Dev {
            dump_dependencies(
                pyproject,
                &local_projects,
                req_file.as_file_mut(),
                DependencyKind::Dev,
            )?;
            dump_dependencies(
                pyproject,
                &local_projects,
                local_req_file.as_file_mut(),
                DependencyKind::Dev,
            )?;
        }
    }

    let exclusions = find_exclusions(&projects)?;
    generate_lockfile(
        output,
        py_ver,
        &workspace.path(),
        req_file.path(),
        lockfile,
        sources,
        lock_options,
        &exclusions,
        &[],
    )?;
    generate_lockfile(
        output,
        py_ver,
        &workspace.path(),
        local_req_file.path(),
        lockfile,
        sources,
        lock_options,
        &exclusions,
        &["--pip-args=--no-deps"],
    )?;

    Ok(())
}

fn format_project_extras<'a>(
    features_by_project: Option<&'a HashMap<String, HashSet<&str>>>,
    project: &PyProject,
) -> Result<Cow<'a, str>, Error> {
    let features: Vec<_> = match features_by_project {
        Some(features_by_project) => features_by_project
            .get(&project.normalized_name()?)
            .map_or(None.into_iter(), |x| Some(x.iter().copied()).into_iter())
            .flatten()
            .chain({
                if project.is_workspace_root() {
                    features_by_project.get("").map(|x| x.iter().copied())
                } else {
                    None
                }
                .into_iter()
                .flatten()
            })
            .collect(),
        None => project.extras().iter().copied().collect(),
    };
    Ok(if features.is_empty() {
        Cow::Borrowed("")
    } else {
        Cow::Owned(format!("[{}]", features.join(",")))
    })
}

fn collect_workspace_features(
    lock_options: &LockOptions,
) -> Option<HashMap<String, HashSet<&str>>> {
    if lock_options.all_features {
        return None;
    }
    let mut features_by_project = HashMap::new();
    for feature in lock_options.features.iter().flat_map(|x| x.split(',')) {
        let feature = feature.trim();
        if feature.is_empty() {
            continue;
        }
        if let Some((project, feature)) = feature.split_once('/') {
            let normalized_project = normalize_package_name(project);
            features_by_project
                .entry(normalized_project)
                .or_insert_with(HashSet::new)
                .insert(feature);
        } else {
            features_by_project
                .entry("".to_string())
                .or_insert_with(HashSet::new)
                .insert(feature);
        }
    }
    Some(features_by_project)
}

fn find_exclusions(projects: &[PyProject]) -> Result<HashSet<Requirement>, Error> {
    let mut rv = HashSet::new();
    for project in projects {
        for dep in project.iter_dependencies(DependencyKind::Excluded) {
            rv.insert(dep.expand(|name: &str| {
                if name == "PROJECT_ROOT" {
                    Some(project.workspace_path().to_string_lossy().to_string())
                } else {
                    std::env::var(name).ok()
                }
            })?);
        }
    }
    Ok(rv)
}

fn dump_dependencies(
    pyproject: &PyProject,
    local_projects: &HashMap<String, String>,
    out: &mut fs::File,
    dep_kind: DependencyKind,
) -> Result<(), Error> {
    for dep in pyproject.iter_dependencies(dep_kind) {
        if let Ok(expanded_dep) = dep.expand(|_| {
            // we actually do not care what it expands to much, for as long
            // as the end result parses
            Some("VARIABLE".into())
        }) {
            if let Some(path) = local_projects.get(&normalize_package_name(&expanded_dep.name)) {
                // if there are extras and we have a local dependency, we just write it
                // out again for pip-compile to pick up the extras.
                // XXX: this drops the marker, but pip-compile already has other
                // problems with markers too: https://github.com/jazzband/pip-tools/issues/826
                if let Some(ref extras) = expanded_dep.extras {
                    writeln!(out, "-e {}[{}]", path, extras.join(","))?;
                }
                continue;
            }
        }
        writeln!(out, "{}", dep)?;
    }
    Ok(())
}

/// Updates the lockfile of the current project.
pub fn update_single_project_lockfile(
    py_ver: &PythonVersion,
    pyproject: &PyProject,
    lock_mode: LockMode,
    lockfile: &Path,
    output: CommandOutput,
    sources: &ExpandedSources,
    lock_options: &LockOptions,
) -> Result<(), Error> {
    if output != CommandOutput::Quiet {
        echo!("Generating {} lockfile: {}", lock_mode, lockfile.display());
    }

    let mut req_file = NamedTempFile::new()?;

    // virtual packages are themselves not installed
    if !pyproject.is_virtual() {
        let features_by_project = collect_workspace_features(lock_options);
        let applicable_extras = format_project_extras(features_by_project.as_ref(), pyproject)?;
        writeln!(
            req_file,
            "-e {}{}",
            make_relative_url(&pyproject.root_path(), &pyproject.workspace_path())?,
            applicable_extras
        )?;
    }

    for dep in pyproject.iter_dependencies(DependencyKind::Normal) {
        writeln!(req_file, "{}", dep)?;
    }
    if lock_mode == LockMode::Dev {
        for dep in pyproject.iter_dependencies(DependencyKind::Dev) {
            writeln!(req_file, "{}", dep)?;
        }
    }

    let exclusions = find_exclusions(std::slice::from_ref(pyproject))?;
    generate_lockfile(
        output,
        py_ver,
        &pyproject.workspace_path(),
        req_file.path(),
        lockfile,
        sources,
        lock_options,
        &exclusions,
        &[],
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn generate_lockfile(
    output: CommandOutput,
    py_ver: &PythonVersion,
    workspace_path: &Path,
    requirements_file_in: &Path,
    lockfile: &Path,
    sources: &ExpandedSources,
    lock_options: &LockOptions,
    exclusions: &HashSet<Requirement>,
    extra_args: &[&str],
) -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let requirements_file = scratch.path().join("requirements.txt");
    if lockfile.is_file() {
        fs::copy(lockfile, &requirements_file)?;
    } else {
        fs::write(&requirements_file, b"")?;
    }

    let pip_compile = get_pip_compile(py_ver, output)?;
    let mut cmd = Command::new(pip_compile);
    cmd.arg("--resolver=backtracking")
        .arg("--no-annotate")
        .arg("--strip-extras")
        .arg("--allow-unsafe")
        .arg("--no-header")
        .arg("--pip-args")
        .arg(format!(
            "--python=\"{}\"",
            get_venv_python_bin(&workspace_path.join(".venv")).display()
        ))
        .arg("-o")
        .arg(&requirements_file)
        .arg(requirements_file_in)
        .current_dir(workspace_path)
        .env("PYTHONWARNINGS", "ignore")
        .env("PROJECT_ROOT", make_project_root_fragment(workspace_path));
    if output == CommandOutput::Verbose {
        cmd.arg("--verbose");
    } else {
        cmd.arg("-q");
    }
    for pkg in &lock_options.update {
        cmd.arg("--upgrade-package");
        cmd.arg(pkg);
    }
    if lock_options.update_all {
        cmd.arg("--upgrade");
    }
    if lock_options.pre {
        cmd.arg("--pre");
    }
    sources.add_as_pip_args(&mut cmd);
    cmd.args(extra_args);
    set_proxy_variables(&mut cmd);
    let status = cmd.status().context("unable to run pip-compile")?;
    if !status.success() {
        bail!("failed to generate lockfile");
    };

    finalize_lockfile(
        &requirements_file,
        lockfile,
        workspace_path,
        exclusions,
        sources,
        lock_options,
    )?;

    Ok(())
}

fn finalize_lockfile(
    generated: &Path,
    out: &Path,
    workspace_root: &Path,
    exclusions: &HashSet<Requirement>,
    sources: &ExpandedSources,
    lock_options: &LockOptions,
) -> Result<(), Error> {
    let mut rv = BufWriter::new(fs::File::create(out)?);
    writeln!(rv, "{}", render!(REQUIREMENTS_HEADER, lock_options))?;

    // only if we are asked to include sources we do that.
    if lock_options.with_sources {
        sources.add_to_lockfile(&mut rv)?;
        writeln!(rv)?;
    }

    for line in fs::read_to_string(generated)?.lines() {
        // we deal with this explicitly.
        if line.trim().is_empty()
            || line.starts_with("--index-url ")
            || line.starts_with("--extra-index-url ")
            || line.starts_with("--find-links ")
        {
            continue;
        }

        if let Some(m) = FILE_EDITABLE_RE.captures(line) {
            let url = Url::parse(&m[1]).context("invalid editable URL generated")?;
            if url.scheme() == "file" {
                let rel_url = make_relative_url(Path::new(url.path()), workspace_root)?;
                writeln!(rv, "-e {}", rel_url)?;
                continue;
            }
        } else if let Ok(ref req) = line.trim().parse::<Requirement>() {
            // TODO: this does not evaluate markers
            if exclusions.iter().any(|x| {
                normalize_package_name(&x.name) == normalize_package_name(&req.name)
                    && (x.version_or_url.is_none() || x.version_or_url == req.version_or_url)
            }) {
                // skip exclusions
                writeln!(rv, "# excluded {}", line)?;
                continue;
            }
        }
        writeln!(rv, "{}", line)?;
    }
    Ok(())
}

pub fn make_project_root_fragment(root: &Path) -> String {
    // XXX: ${PROJECT_ROOT} is supposed to be used in the context of file:///
    // so let's make sure it is url escaped.  This is pretty hacky but
    // good enough for now.
    // No leading slash to fit with file:///${PROJECT_ROOT} convention
    root.to_string_lossy()
        .trim_start_matches('/')
        .replace(' ', "%20")
}

fn make_relative_url(path: &Path, base: &Path) -> Result<String, Error> {
    // TODO: consider using ${PROJECT_ROOT} here which is what pdm does or make-req prints
    let rv = pathdiff::diff_paths(path, base).ok_or_else(|| {
        anyhow!(
            "unable to create relative path from {} to {}",
            base.display(),
            path.display()
        )
    })?;
    if rv == Path::new("") {
        Ok("file:.".into())
    } else {
        // XXX: there might be a better way to do this, but this appears to be enough
        // to make this work for now.
        let mut buf = String::new();
        for chunk in url::form_urlencoded::byte_serialize(rv.to_string_lossy().as_bytes()) {
            buf.push_str(
                &chunk
                    .replace('+', "%20")
                    .replace("%2F", "/")
                    .replace("%5C", "/"),
            );
        }
        Ok(format!("file:{}", buf))
    }
}

#[test]
fn test_make_relativec_url() {
    assert_eq!(
        make_relative_url(Path::new("foo/bar/baz blah"), Path::new("foo")).unwrap(),
        "file:bar/baz%20blah"
    );
    assert_eq!(
        make_relative_url(Path::new("/foo"), Path::new("/foo")).unwrap(),
        "file:."
    );
}
