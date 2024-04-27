use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::{env, fmt, fs};

use anyhow::{anyhow, bail, Context, Error};
use clap::ValueEnum;
use minijinja::render;
use once_cell::sync::Lazy;
use pep508_rs::Requirement;
use regex::Regex;
use serde::Serialize;
use tempfile::NamedTempFile;
use url::Url;

use crate::config::Config;
use crate::piptools::{get_pip_compile, get_pip_tools_version, PipToolsVersion};
use crate::pyproject::{
    normalize_package_name, DependencyKind, ExpandedSources, PyProject, Workspace,
};
use crate::sources::py::PythonVersion;
use crate::utils::{set_proxy_variables, CommandOutput, IoPathContext};
use crate::uv::{UvBuilder, UvPackageUpgrade};

static FILE_EDITABLE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^-e (file://.*?)\s*$").unwrap());
static DEP_COMMENT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^    # (?:(via)|(?:via (.*?))|(?:  (.*?)))$").unwrap());
static REQUIREMENTS_HEADER: &str = r#"# generated by rye
# use `rye lock` or `rye sync` to update this lockfile
#
# last locked with the following flags:
#   pre: {{ lock_options.pre|tojson }}
#   features: {{ lock_options.features|tojson }}
#   all-features: {{ lock_options.all_features|tojson }}
#   with-sources: {{ lock_options.with_sources|tojson }}

"#;
static PARAM_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^#   (pre|features|all-features|with-sources):\s*(.*?)$").unwrap());

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

/// Keyring provider type to use for credential lookup.
#[derive(ValueEnum, Copy, Clone, Serialize, Debug, Default, PartialEq)]
#[value(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum KeyringProvider {
    /// Do not use keyring for credential lookup.
    #[default]
    Disabled,
    /// Use the `keyring` command for credential lookup.
    Subprocess,
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
    /// Do not reuse (reset) prior lock options.
    pub reset: bool,
}

impl LockOptions {
    /// Writes the lock options as header.
    pub fn write_header<W: Write>(&self, mut w: W) -> Result<(), Error> {
        writeln!(w, "{}", render!(REQUIREMENTS_HEADER, lock_options => self))?;
        Ok(())
    }

    /// Restores lock options from a requirements file.
    ///
    /// This also applies overrides from the command line.
    pub fn restore<'o>(s: &str, opts: &'o LockOptions) -> Result<Cow<'o, LockOptions>, Error> {
        // nothing to do here
        if opts.reset {
            return Ok(Cow::Borrowed(opts));
        }

        let mut rv = opts.clone();
        for line in s
            .lines()
            .skip_while(|x| *x != "# last locked with the following flags:")
        {
            if let Some(m) = PARAM_RE.captures(line) {
                let value = &m[2];
                match &m[1] {
                    "pre" => rv.pre = rv.pre || serde_json::from_str(value)?,
                    "features" => {
                        if rv.features.is_empty() {
                            rv.features = serde_json::from_str(value)?;
                        }
                    }
                    "all-features" => {
                        rv.all_features = rv.all_features || serde_json::from_str(value)?
                    }
                    "with-sources" => {
                        rv.with_sources = rv.with_sources || serde_json::from_str(value)?
                    }
                    _ => unreachable!(),
                }
            }
        }

        if rv.all_features {
            rv.features = Vec::new();
        }

        Ok(Cow::Owned(rv))
    }
}

/// Creates lockfiles for all projects in the workspace.
#[allow(clippy::too_many_arguments)]
pub fn update_workspace_lockfile(
    py_ver: &PythonVersion,
    workspace: &Arc<Workspace>,
    lock_mode: LockMode,
    lockfile: &Path,
    output: CommandOutput,
    sources: &ExpandedSources,
    lock_options: &LockOptions,
    keyring_provider: KeyringProvider,
) -> Result<(), Error> {
    echo!(if output, "Generating {} lockfile: {}", lock_mode, lockfile.display());

    let lock_options = restore_lock_options(lockfile, lock_options)?;
    let features_by_project = collect_workspace_features(&lock_options);
    let mut req_file = NamedTempFile::new()?;

    let mut local_projects = HashMap::new();
    let mut projects = Vec::new();
    for pyproject_result in workspace.iter_projects() {
        let pyproject = pyproject_result?;
        let rel_url = make_relative_url(&pyproject.root_path(), &workspace.path())?;
        let applicable_extras = format_project_extras(features_by_project.as_ref(), &pyproject)?;

        // virtual packages are not installed
        if !pyproject.is_virtual() {
            writeln!(req_file, "-e {}{}", rel_url, applicable_extras)?;
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
        }
    }

    req_file.flush()?;

    let exclusions = find_exclusions(&projects)?;
    generate_lockfile(
        output,
        py_ver,
        &workspace.path(),
        req_file.path(),
        lockfile,
        sources,
        &lock_options,
        &exclusions,
        true,
        keyring_provider,
    )?;

    Ok(())
}

/// Tries to restore the lock options from the given lockfile.
fn restore_lock_options<'o>(
    lockfile: &Path,
    lock_options: &'o LockOptions,
) -> Result<Cow<'o, LockOptions>, Error> {
    if lockfile.is_file() {
        let requirements = fs::read_to_string(lockfile)?;
        Ok(LockOptions::restore(&requirements, lock_options)?)
    } else {
        Ok(Cow::Borrowed(lock_options))
    }
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
#[allow(clippy::too_many_arguments)]
pub fn update_single_project_lockfile(
    py_ver: &PythonVersion,
    pyproject: &PyProject,
    lock_mode: LockMode,
    lockfile: &Path,
    output: CommandOutput,
    sources: &ExpandedSources,
    lock_options: &LockOptions,
    keyring_provider: KeyringProvider,
) -> Result<(), Error> {
    echo!(if output, "Generating {} lockfile: {}", lock_mode, lockfile.display());

    let lock_options = restore_lock_options(lockfile, lock_options)?;
    let mut req_file = NamedTempFile::new()?;

    // virtual packages are themselves not installed
    if !pyproject.is_virtual() {
        let features_by_project = collect_workspace_features(&lock_options);
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

    req_file.flush()?;

    let exclusions = find_exclusions(std::slice::from_ref(pyproject))?;
    generate_lockfile(
        output,
        py_ver,
        &pyproject.workspace_path(),
        req_file.path(),
        lockfile,
        sources,
        &lock_options,
        &exclusions,
        false,
        keyring_provider,
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
    no_deps: bool,
    keyring_provider: KeyringProvider,
) -> Result<(), Error> {
    let use_uv = Config::current().use_uv();
    let scratch = tempfile::tempdir()?;
    let requirements_file = scratch.path().join("requirements.txt");
    if lockfile.is_file() {
        fs::copy(lockfile, &requirements_file)
            .path_context(&requirements_file, "unable to restore requirements file")?;
    } else if !use_uv {
        fs::write(&requirements_file, b"").path_context(
            &requirements_file,
            "unable to write empty requirements file",
        )?;
    };

    if use_uv {
        let upgrade = {
            if lock_options.update_all {
                UvPackageUpgrade::All
            } else if !lock_options.update.is_empty() {
                UvPackageUpgrade::Packages(lock_options.update.clone())
            } else {
                UvPackageUpgrade::Nothing
            }
        };

        UvBuilder::new()
            .with_output(output.quieter())
            .with_sources(sources.clone())
            .with_workdir(workspace_path)
            .ensure_exists()?
            .lockfile(
                py_ver,
                requirements_file_in,
                &requirements_file,
                lock_options.pre,
                env::var("__RYE_UV_EXCLUDE_NEWER").ok(),
                upgrade,
                keyring_provider,
            )?;
    } else {
        if keyring_provider != KeyringProvider::Disabled {
            bail!("--keyring-provider option is only supported with uv");
        }
        let mut cmd = Command::new(get_pip_compile(py_ver, output)?);
        // legacy pip tools requires some extra parameters
        if get_pip_tools_version(py_ver) == PipToolsVersion::Legacy {
            cmd.arg("--resolver=backtracking");
        }
        cmd.arg("--strip-extras")
            .arg("--allow-unsafe")
            .arg("--no-header")
            .arg("--annotate")
            .arg("--pip-args")
            .arg(format!(
                "--python-version=\"{}.{}.{}\"{}",
                py_ver.major,
                py_ver.minor,
                py_ver.patch,
                if no_deps { " --no-deps" } else { "" }
            ));
        if lock_options.pre {
            cmd.arg("--pre");
        }

        cmd.arg(if output == CommandOutput::Verbose {
            "--verbose"
        } else {
            "-q"
        })
        .arg("-o")
        .arg(&requirements_file)
        .arg(requirements_file_in)
        .current_dir(workspace_path)
        .env("PYTHONWARNINGS", "ignore")
        .env("PROJECT_ROOT", make_project_root_fragment(workspace_path));

        for pkg in &lock_options.update {
            cmd.arg("--upgrade-package");
            cmd.arg(pkg);
        }
        if lock_options.update_all {
            cmd.arg("--upgrade");
        }
        sources.add_as_pip_args(&mut cmd);
        set_proxy_variables(&mut cmd);
        let status = cmd.status().context("unable to run pip-compile")?;
        if !status.success() {
            bail!("failed to generate lockfile");
        };
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
    let mut rv =
        BufWriter::new(fs::File::create(out).path_context(out, "unable to finalize lockfile")?);
    lock_options.write_header(&mut rv)?;

    // only if we are asked to include sources we do that.
    if lock_options.with_sources {
        sources.add_to_lockfile(&mut rv)?;
        writeln!(rv)?;
    }

    for line in fs::read_to_string(generated)
        .path_context(generated, "unable to parse resolver output")?
        .lines()
    {
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
                writeln!(rv, "# {} (excluded)", line)?;
                continue;
            }
        } else if let Some(m) = DEP_COMMENT_RE.captures(line) {
            if let Some(dep) = m.get(2).or_else(|| m.get(3)).map(|x| x.as_str()) {
                if !dep.starts_with("-r ") {
                    // we cannot tell today based on the output where this comes from.  This
                    // can show up because it's a root dependency, because it's a dev dependency
                    // or in some cases just because we declared it as a duplicate.
                    writeln!(rv, "    # via {}", dep)?;
                }
            };
            continue;
        } else if line.starts_with('#') {
            continue;
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
