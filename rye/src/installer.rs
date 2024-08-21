use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

use anyhow::{bail, Context, Error};
use console::style;
use once_cell::sync::Lazy;
use pep508_rs::{Requirement, VersionOrUrl};
use regex::Regex;
use same_file::is_same_file;
use url::Url;

use crate::bootstrap::{ensure_self_venv, fetch, FetchOptions};
use crate::config::Config;
use crate::consts::VENV_BIN;
use crate::lock::KeyringProvider;
use crate::platform::get_app_dir;
use crate::pyproject::{normalize_package_name, read_venv_marker, ExpandedSources};
use crate::sources::py::PythonVersionRequest;
use crate::sync::{create_virtualenv, VenvMarker};
use crate::utils::{
    get_short_executable_name, get_venv_python_bin, is_executable, symlink_file, CommandOutput,
    IoPathContext,
};
use crate::uv::{UvBuilder, UvInstallOptions};

const FIND_SCRIPT_SCRIPT: &str = r#"
import os
import re
import sys
import json

if sys.version_info >= (3, 8):
    from importlib.metadata import distribution, PackageNotFoundError
else:
    from importlib_metadata import distribution, PackageNotFoundError

_package_re = re.compile('(?i)^([a-z0-9._-]+)')

result = {}

def dump_all(dist, root=False):
    rv = []
    for file in dist.files or ():
        rv.append(os.path.normpath(dist.locate_file(file)))
    result["" if root else dist.name] = rv
    req = []
    for r in dist.requires or ():
        name = _package_re.match(r)
        if name is not None:
            req.append(name.group())
    return req

root = sys.argv[1]
to_resolve = [root]
seen = set()
while to_resolve:
    try:
        d = to_resolve.pop()
        dist = distribution(d)
    except Exception:
        continue
    if dist.name in seen:
        continue
    seen.add(dist.name)
    to_resolve.extend(dump_all(dist, root=d==root))

print(json.dumps(result))
"#;
static SUCCESSFULLY_DOWNLOADED_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new("(?m)^Successfully downloaded (.*?)$").unwrap());

#[derive(Eq, PartialEq)]
pub struct ToolInfo {
    pub version: String,
    pub scripts: Vec<String>,
    pub venv_marker: Option<VenvMarker>,
    pub valid: bool,
}

impl ToolInfo {
    pub fn new(
        version: String,
        scripts: Vec<String>,
        venv_marker: Option<VenvMarker>,
        valid: bool,
    ) -> Self {
        Self {
            version,
            scripts,
            venv_marker,
            valid,
        }
    }
}

const TOOL_VERSION_SCRIPT: &str = r#"
import sys
from importlib.metadata import version

tool_name = sys.argv[1]
print(version(tool_name))
"#;

pub fn install(
    requirement: Requirement,
    py_ver: &PythonVersionRequest,
    force: bool,
    include_deps: &[String],
    extra_requirements: &[Requirement],
    output: CommandOutput,
    keyring_provider: KeyringProvider,
) -> Result<(), Error> {
    let config = Config::current();
    let sources = ExpandedSources::from_sources(&config.sources()?)?;
    let app_dir = get_app_dir();
    let shim_dir = app_dir.join("shims");
    let self_venv = ensure_self_venv(output)?;
    let tool_dir = app_dir.join("tools");
    let include_deps = include_deps
        .iter()
        .map(|x| normalize_package_name(x))
        .collect::<Vec<_>>();

    let target_venv_path = tool_dir.join(normalize_package_name(&requirement.name));
    if target_venv_path.is_dir() && !force {
        bail!("package already installed");
    }
    let py = get_venv_python_bin(&target_venv_path);
    let target_venv_bin_path = target_venv_path.join(VENV_BIN);

    uninstall_helper(&target_venv_path, &shim_dir)?;

    // make sure we have a compatible python version
    let py_ver = fetch(py_ver, FetchOptions::with_output(output))?;

    create_virtualenv(
        output,
        &self_venv,
        &py_ver,
        &target_venv_path,
        requirement.name.as_str(),
    )?;

    let result = UvBuilder::new()
        .with_output(output.quieter())
        .with_sources(sources)
        .ensure_exists()?
        .venv(&target_venv_path, &py, &py_ver, None)?
        .with_output(output)
        .install(
            &requirement,
            UvInstallOptions {
                importlib_workaround: py_ver.major == 3 && py_ver.minor == 7,
                extras: extra_requirements.to_vec(),
                refresh: force,
                keyring_provider,
            },
        );
    if result.is_err() {
        uninstall_helper(&target_venv_path, &shim_dir)?;
        return result;
    }

    let out = Command::new(py)
        .arg("-c")
        .arg(FIND_SCRIPT_SCRIPT)
        .arg(&requirement.name)
        .stdout(Stdio::piped())
        .output()
        .context("unable to dump package manifest from installed package")?;
    let all_files: BTreeMap<String, Vec<PathBuf>> = serde_json::from_slice(&out.stdout)
        .with_context(|| {
            format!(
                "failed to resolve manifest\n{}",
                String::from_utf8_lossy(&out.stderr)
            )
        })?;

    let mut installed = Vec::new();
    let mut scripts_found = Vec::new();
    if let Some(files) = all_files.get("") {
        installed.extend(install_scripts(files, &target_venv_bin_path, &shim_dir)?);
    }

    for (package, files) in all_files.iter() {
        if package.is_empty() {
            continue;
        }
        if include_deps.contains(&normalize_package_name(package)) {
            installed.extend(install_scripts(files, &target_venv_bin_path, &shim_dir)?);
        } else {
            let scripts = find_scripts(files, &target_venv_bin_path);
            if !scripts.is_empty() {
                scripts_found.push((package, scripts));
            }
        }
    }

    if !scripts_found.is_empty()
        && output != CommandOutput::Quiet
        && (installed.is_empty() || output == CommandOutput::Verbose)
    {
        echo!(
            "{}",
            style("Found additional non installed scripts in dependencies:").yellow()
        );
        scripts_found.sort();
        for (package, scripts) in scripts_found.iter() {
            echo!("{}:", style(package).green());
            for script in scripts {
                echo!("  - {}", style(script).cyan());
            }
        }
        echo!("To install scripts from these packages pass the appropriate --include-dep");
    }

    if output != CommandOutput::Quiet {
        echo!();
        if installed.is_empty() {
            warn!("installed package did not expose any scripts")
        } else {
            echo!("Installed scripts:");
            for script in installed {
                echo!("  - {}", style(script).cyan());
            }
            if output != CommandOutput::Verbose && !scripts_found.is_empty() {
                echo!();
                echo!(
                    "note: {}",
                    style("additional scripts were encountered in non-installed dependencies.")
                        .dim()
                );
            }
        }
    }

    Ok(())
}

fn find_scripts(files: &[PathBuf], target_venv_bin_path: &Path) -> Vec<String> {
    let mut rv = Vec::new();
    for file in files {
        if let Ok(rest) = file.strip_prefix(target_venv_bin_path) {
            rv.push(rest.to_string_lossy().to_string());
        }
    }
    rv
}

fn install_scripts(
    files: &[PathBuf],
    target_venv_bin_path: &Path,
    shim_dir: &Path,
) -> Result<Vec<String>, Error> {
    let mut rv = Vec::new();
    for file in files {
        if let Ok(rest) = file.strip_prefix(target_venv_bin_path) {
            // In some cases we are given paths here which point to sub-folders of the
            // script/bin folder.  For instance in some cases it has been shown that
            // __pycache__/something.pyc shows up there.  These are obviously not good
            // targets to link as they would never show up via PATH discovery.  Skip
            // over these.
            //
            // Also do not try to link things which are not considered executables on
            // this operating system.
            if !rest.parent().map_or(true, |x| x == Path::new("")) || !is_executable(file) {
                continue;
            }

            let shim_target = shim_dir.join(rest);

            // on windows we want to fall back to hardlinks.  That might be problematic in
            // some cases, but it should work for most cases where setuptools or other
            // systems created exe files.  Caveat: uninstallation currently does not work
            // when hardlinks are used.
            #[cfg(windows)]
            {
                if symlink_file(file, &shim_target).is_err() {
                    fs::hard_link(file, &shim_target)
                        .path_context(file, "unable to symlink tool")?;
                }
            }
            #[cfg(unix)]
            {
                symlink_file(file, shim_target).path_context(file, "unable to symlink tool")?;
            }
            rv.push(get_short_executable_name(file));
        }
    }
    Ok(rv)
}

pub fn uninstall(package: &str, output: CommandOutput) -> Result<(), Error> {
    let app_dir = get_app_dir();
    let shim_dir = app_dir.join("shims");
    let tool_dir = app_dir.join("tools");
    let target_venv_path = tool_dir.join(normalize_package_name(package));
    if !target_venv_path.is_dir() {
        echo!("{} is not installed", style(package).cyan());
        return Ok(());
    }

    uninstall_helper(&target_venv_path, &shim_dir)
        .with_context(|| format!("unable to uninstall {}", target_venv_path.display()))?;
    if output != CommandOutput::Quiet {
        echo!("Uninstalled {}", style(package).cyan());
    }
    Ok(())
}

pub fn list_installed_tools() -> Result<HashMap<String, ToolInfo>, Error> {
    let app_dir = get_app_dir();
    let shim_dir = app_dir.join("shims");
    let tool_dir = app_dir.join("tools");
    if !tool_dir.is_dir() {
        return Ok(HashMap::new());
    }

    let mut rv = HashMap::new();
    for folder in fs::read_dir(&tool_dir).path_context(&tool_dir, "unable to enumerate tools")? {
        let folder = folder?;
        if !folder.file_type()?.is_dir() {
            continue;
        }
        let tool_name = folder.file_name().to_string_lossy().to_string();
        let target_venv_bin_path = folder.path().join(VENV_BIN);
        let venv_marker = read_venv_marker(&folder.path());

        let mut scripts = Vec::new();
        for script in fs::read_dir(target_venv_bin_path.clone())
            .path_context(&target_venv_bin_path, "unable to enumerate scripts")?
        {
            let script = script?;
            let script_path = script.path();
            if let Some(base_name) = script_path.file_name() {
                let shim_path = shim_dir.join(base_name);
                if let Ok(true) = is_same_file(&shim_path, &script_path) {
                    scripts.push(get_short_executable_name(&script_path));
                }
            }
        }

        let output = Command::new(target_venv_bin_path.join("python"))
            .arg("-c")
            .arg(TOOL_VERSION_SCRIPT)
            .arg(tool_name.clone())
            .stdout(Stdio::piped())
            .output();
        let valid = output.is_ok();
        let tool_version = match output {
            Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
            Err(_) => String::new(),
        };

        rv.insert(
            tool_name,
            ToolInfo::new(tool_version, scripts, venv_marker, valid),
        );
    }

    Ok(rv)
}

fn uninstall_helper(target_venv_path: &Path, shim_dir: &Path) -> Result<(), Error> {
    let target_venv_bin_path = target_venv_path.join(VENV_BIN);
    if !target_venv_bin_path.is_dir() {
        return Ok(());
    }

    for script in fs::read_dir(&target_venv_bin_path)
        .path_context(&target_venv_bin_path, "unable to enumerate scripts")?
    {
        let script = script?;
        if let Some(base_name) = script.path().file_name() {
            let shim_path = shim_dir.join(base_name);
            if let Ok(true) = is_same_file(&shim_path, script.path()) {
                fs::remove_file(&shim_path).ok();
            }
        }
    }

    fs::remove_dir_all(target_venv_path).ok();

    Ok(())
}

/// Super hacky way to ensure that if something points to a local path,
/// we can figure out what the actual requirement name is.
pub fn resolve_local_requirement(
    maybe_path: &Path,
    output: CommandOutput,
) -> Result<Option<Requirement>, Error> {
    let self_venv = ensure_self_venv(output)?;
    if !maybe_path.exists() {
        return Ok(None);
    }

    let output = Command::new(self_venv.join("bin/pip"))
        .arg("download")
        .arg("--no-deps")
        .arg("--")
        .arg(maybe_path)
        .output()?;
    let output = String::from_utf8_lossy(&output.stdout);
    if let Some(c) = SUCCESSFULLY_DOWNLOADED_RE.captures(&output) {
        let version_or_url = Some(VersionOrUrl::Url(
            match Url::from_file_path(env::current_dir()?.join(maybe_path)) {
                Ok(url) => url,
                Err(()) => bail!("invalid path reference"),
            },
        ));
        let name = c[1].trim().to_string();
        Ok(Some(Requirement {
            extras: None,
            name,
            version_or_url,
            marker: None,
        }))
    } else {
        Ok(None)
    }
}
