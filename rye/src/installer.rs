use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

use anyhow::{bail, Context, Error};
use console::style;
use once_cell::sync::Lazy;
use pep508_rs::{Requirement, VersionOrUrl};
use regex::Regex;
use url::Url;

use crate::bootstrap::{ensure_self_venv, fetch};
use crate::consts::VENV_BIN;
use crate::platform::get_app_dir;
use crate::pyproject::normalize_package_name;
use crate::sources::PythonVersionRequest;
use crate::sync::create_virtualenv;
use crate::utils::{get_venv_python_bin, symlink_file, CommandOutput};

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

pub fn install(
    requirement: Requirement,
    py_ver: &PythonVersionRequest,
    force: bool,
    include_deps: &[String],
    output: CommandOutput,
) -> Result<(), Error> {
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
    let py_ver = fetch(py_ver, output)?;

    create_virtualenv(output, &self_venv, &py_ver, &target_venv_path)?;

    let mut cmd = Command::new(self_venv.join(VENV_BIN).join("pip"));
    cmd.arg("--python")
        .arg(&py)
        .arg("install")
        .env("PYTHONWARNINGS", "ignore");
    if output == CommandOutput::Verbose {
        cmd.arg("--verbose");
    } else {
        if output == CommandOutput::Quiet {
            cmd.arg("-q");
        }
        cmd.env("PYTHONWARNINGS", "ignore");
    }
    cmd.arg("--")
        .arg(&requirement.to_string())
        .arg("importlib-metadata==6.6.0; python_version==\"3.7\"");

    let status = cmd.status()?;
    if !status.success() {
        bail!("tool installation failed");
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
        eprintln!(
            "{}",
            style("Found additional non installed scripts in dependencies:").yellow()
        );
        scripts_found.sort();
        for (package, scripts) in scripts_found.iter() {
            eprintln!("{}:", style(package).green());
            for script in scripts {
                eprintln!("  - {}", style(script).cyan());
            }
        }
        eprintln!("To install scripts from these packages pass the appropriate --include-dep");
    }

    if output != CommandOutput::Quiet {
        eprintln!();
        if installed.is_empty() {
            eprintln!(
                "{}",
                style("warning: installed package did not expose any scripts").red()
            );
        } else {
            eprintln!("Installed scripts:");
            for script in installed {
                eprintln!("  - {}", style(script).cyan());
            }
            if output != CommandOutput::Verbose && !scripts_found.is_empty() {
                eprintln!();
                eprintln!(
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
            let shim_target = shim_dir.join(rest);
            symlink_file(file, shim_target)
                .with_context(|| format!("unable to symlink tool to {}", file.display()))?;
            rv.push(rest.to_string_lossy().to_string());
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
        eprintln!("{} is not installed", style(package).cyan());
        return Ok(());
    }

    uninstall_helper(&target_venv_path, &shim_dir)
        .with_context(|| format!("unable to uninstall {}", target_venv_path.display()))?;
    if output != CommandOutput::Quiet {
        eprintln!("Uninstalled {}", style(package).cyan());
    }
    Ok(())
}

fn uninstall_helper(target_venv_path: &Path, shim_dir: &Path) -> Result<(), Error> {
    fs::remove_dir_all(target_venv_path).ok();

    for script in fs::read_dir(shim_dir)? {
        let script = script?;
        if !script.path().is_symlink() {
            continue;
        }
        if let Ok(target) = fs::read_link(&script.path()) {
            if target.strip_prefix(target_venv_path).is_ok() {
                fs::remove_file(&script.path())?;
            }
        }
    }

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
