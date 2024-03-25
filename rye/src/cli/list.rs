use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Error};
use clap::Parser;

use crate::bootstrap::ensure_self_venv;
use crate::config::Config;
use crate::consts::VENV_BIN;
use crate::pyproject::PyProject;
use crate::utils::{get_venv_python_bin, CommandOutput};
use crate::uv::{UvBuilder, UvWithVenv};

/// Prints the currently installed packages.
#[derive(Parser, Debug)]
pub struct Args {
    /// Use this pyproject.toml file
    #[arg(long, value_name = "PYPROJECT_TOML")]
    pub(crate) pyproject: Option<PathBuf>,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let project = PyProject::load_or_discover(cmd.pyproject.as_deref())?;
    let python = get_venv_python_bin(&project.venv_path());
    if !python.is_file() {
        return Ok(());
    }
    let self_venv = ensure_self_venv(CommandOutput::Normal)?;

    if Config::current().use_uv() {
        let uv = UvBuilder::new()
            .with_output(CommandOutput::Normal)
            .ensure_exists()?;
        if !project.rye_managed() {
            UvWithVenv::new(uv, &project.venv_path(), &project.venv_python_version()?).freeze()?;
        } else {
            uv.venv(
                &project.venv_path(),
                &python,
                &project.venv_python_version()?,
                None,
            )?
            .freeze()?;
        }
    } else {
        let status = Command::new(self_venv.join(VENV_BIN).join("pip"))
            .arg("--python")
            .arg(&python)
            .arg("freeze")
            .env("PYTHONWARNINGS", "ignore")
            .env("PIP_DISABLE_PIP_VERSION_CHECK", "1")
            .status()?;

        if !status.success() {
            bail!("failed to print dependencies via pip");
        }
    };

    Ok(())
}
