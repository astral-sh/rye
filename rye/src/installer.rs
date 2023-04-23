use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{bail, Error};
use console::style;
use pep508_rs::Requirement;

use crate::bootstrap::ensure_self_venv;
use crate::config::get_app_dir;
use crate::pyproject::normalize_package_name;
use crate::sources::PythonVersion;
use crate::sync::create_virtualenv;
use crate::utils::CommandOutput;

const FIND_SCRIPT_SCRIPT: &str = r#"
import os
import sys
from importlib.metadata import distribution

dist = distribution(sys.argv[1])
for file in dist.files:
    print(os.path.normpath(dist.locate_file(file)))
"#;

pub fn install(
    requirement: Requirement,
    py_ver: PythonVersion,
    force: bool,
    output: CommandOutput,
) -> Result<(), Error> {
    let app_dir = get_app_dir()?;
    let shim_dir = app_dir.join("shims");
    let self_venv = ensure_self_venv(output)?;
    let tool_dir = app_dir.join("tools");

    let target_venv_path = tool_dir.join(normalize_package_name(&requirement.name));
    if target_venv_path.is_dir() && !force {
        bail!("package already installed");
    }
    let target_venv_bin_path = target_venv_path.join("bin");

    uninstall_helper(&target_venv_path, &shim_dir)?;
    create_virtualenv(output, &self_venv, py_ver, &target_venv_path)?;

    let mut cmd = Command::new(&self_venv.join("bin/pip"));
    cmd.arg("--python")
        .arg(&target_venv_bin_path.join("python"))
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
    cmd.arg("--").arg(&requirement.to_string());

    let status = cmd.status()?;
    if !status.success() {
        bail!("tool installation failed");
    }

    let out = Command::new(&target_venv_bin_path.join("python"))
        .arg("-c")
        .arg(FIND_SCRIPT_SCRIPT)
        .arg(&requirement.name)
        .stdout(Stdio::piped())
        .output()?;
    let files = std::str::from_utf8(&out.stdout)?
        .lines()
        .map(Path::new)
        .collect::<Vec<_>>();

    for file in files {
        if let Ok(rest) = file.strip_prefix(&target_venv_bin_path) {
            let shim_target = shim_dir.join(rest);
            symlink(file, shim_target)?;
            if output != CommandOutput::Quiet {
                eprintln!("installed script {}", style(rest.display()).cyan());
            }
        }
    }

    Ok(())
}

pub fn uninstall(package: &str, output: CommandOutput) -> Result<(), Error> {
    let app_dir = get_app_dir()?;
    let shim_dir = app_dir.join("shims");
    let tool_dir = app_dir.join("tools");
    let target_venv_path = tool_dir.join(normalize_package_name(package));
    if !target_venv_path.is_dir() {
        eprintln!("{} is not installed", style(package).cyan());
        return Ok(());
    }

    uninstall_helper(&target_venv_path, &shim_dir)?;
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
