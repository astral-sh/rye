use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Error};

use crate::bootstrap::ensure_self_venv;
use crate::consts::VENV_BIN;
use crate::platform::get_app_dir;
use crate::sources::PythonVersion;
use crate::sync::create_virtualenv;
use crate::utils::{get_venv_python_bin, CommandOutput};

// When changing these, also update `SELF_VERSION` in bootstrap.rs to ensure
// that the internals are re-created.
pub const LATEST_PIP: &str = "pip==23.3.2";
const PIP_TOOLS_LATEST_REQ: &[&str] = &[LATEST_PIP, "pip-tools==7.3.0"];
const PIP_TOOLS_LEGACY_REQ: &[&str] = &["pip==22.2.0", "pip-tools==6.14.0"];

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// Which version of piptools are in use?
pub enum PipToolsVersion {
    Latest,
    Legacy,
}

impl PipToolsVersion {
    fn requirements(&self) -> &'static [&'static str] {
        match *self {
            PipToolsVersion::Latest => PIP_TOOLS_LATEST_REQ,
            PipToolsVersion::Legacy => PIP_TOOLS_LEGACY_REQ,
        }
    }
}

fn get_pip_tools_bin(py_ver: &PythonVersion, output: CommandOutput) -> Result<PathBuf, Error> {
    let self_venv = ensure_self_venv(output)?;
    let venv = get_pip_tools_venv_path(py_ver);

    let py = get_venv_python_bin(&venv);
    let version = get_pip_tools_version(py_ver);

    // if we have a python interpreter in the given path, let's use it
    if venv.join(&py).is_file() {
        return Ok(venv);
    }

    // if however for some reason the virtualenv itself is already a folder
    // it usually means that the symlink to the python is bad now.  This can
    // happen if someone wiped the toolchain of the pip-tools version.  In
    // that case wipe it first.
    if venv.is_dir() {
        fs::remove_dir_all(&venv).context("unable to wipe old virtualenv for pip-tools")?;
    }

    if output != CommandOutput::Quiet {
        echo!("Creating virtualenv for pip-tools");
    }
    create_virtualenv(output, &self_venv, py_ver, &venv, "pip-tools")?;

    let mut cmd = Command::new(self_venv.join(VENV_BIN).join("pip"));
    cmd.arg("--python")
        .arg(&py)
        .arg("install")
        .arg("--upgrade")
        .args(version.requirements())
        .arg("-q")
        .env("PIP_DISABLE_PIP_VERSION_CHECK", "1");
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

pub fn get_pip_tools_version(py_ver: &PythonVersion) -> PipToolsVersion {
    if py_ver.major == 3 && py_ver.minor == 7 {
        PipToolsVersion::Legacy
    } else {
        PipToolsVersion::Latest
    }
}

pub fn get_pip_tools_venv_path(py_ver: &PythonVersion) -> PathBuf {
    let key = format!("{}@{}.{}", py_ver.name, py_ver.major, py_ver.minor);
    get_app_dir().join("pip-tools").join(key)
}

pub fn get_pip_sync(py_ver: &PythonVersion, output: CommandOutput) -> Result<PathBuf, Error> {
    Ok(get_pip_tools_bin(py_ver, output)?
        .join(VENV_BIN)
        .join("pip-sync"))
}

pub fn get_pip_compile(py_ver: &PythonVersion, output: CommandOutput) -> Result<PathBuf, Error> {
    Ok(get_pip_tools_bin(py_ver, output)?
        .join(VENV_BIN)
        .join("pip-compile"))
}
