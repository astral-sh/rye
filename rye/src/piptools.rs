use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Error};

use crate::bootstrap::ensure_self_venv;
use crate::config::get_app_dir;
use crate::sources::PythonVersion;
use crate::sync::create_virtualenv;
use crate::utils::CommandOutput;

const PIP_TOOLS_VERSION: &str = "pip-tools==6.13.0";

fn get_pip_tools_bin(py_ver: &PythonVersion, output: CommandOutput) -> Result<PathBuf, Error> {
    let self_venv = ensure_self_venv(output)?;
    let key = format!("py{}.{}", py_ver.major, py_ver.minor);
    let venv = get_app_dir()?.join("pip-tools").join(key);
    let py = venv.join("bin/python");
    if venv.join(&py).is_file() {
        return Ok(venv);
    }

    if output != CommandOutput::Quiet {
        eprintln!("Creating virtualenv for pip-tools");
    }
    create_virtualenv(output, &self_venv, py_ver, &venv)?;

    let mut cmd = Command::new(self_venv.join("bin/pip"));
    cmd.arg("--python")
        .arg(&py)
        .arg("install")
        .arg(PIP_TOOLS_VERSION)
        .arg("-q");
    if output == CommandOutput::Verbose {
        cmd.arg("--verbose");
    } else {
        cmd.arg("--quiet");
        cmd.env("PYTHONWARNINGS", "ignore");
    }
    let status = cmd.status().context("unable to install pip-tools")?;
    if !status.success() {
        bail!("failed to initialize pip-tools venv (install dependencies)");
    }
    Ok(venv)
}

pub fn get_pip_sync(py_ver: &PythonVersion, output: CommandOutput) -> Result<PathBuf, Error> {
    Ok(get_pip_tools_bin(py_ver, output)?.join("bin/pip-sync"))
}

pub fn get_pip_compile(py_ver: &PythonVersion, output: CommandOutput) -> Result<PathBuf, Error> {
    Ok(get_pip_tools_bin(py_ver, output)?.join("bin/pip-compile"))
}
