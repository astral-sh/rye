use std::path::PathBuf;

use anyhow::{bail, Error};
use clap::Parser;

use crate::pyproject::{read_venv_marker, PyProject};
use crate::utils::{get_venv_python_bin, CommandOutput};
use crate::uv::UvBuilder;

/// Prints the currently installed packages.
#[derive(Parser, Debug)]
pub struct Args {
    /// Use this pyproject.toml file
    #[arg(long, value_name = "PYPROJECT_TOML")]
    pub(crate) pyproject: Option<PathBuf>,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let project = PyProject::load_or_discover(cmd.pyproject.as_deref())?;
    let venv = project.venv_path();
    if venv.is_dir() {
        if read_venv_marker(&venv).is_some() {
        } else {
            bail!("virtualenv is not managed by rye.");
        }
    }
    let python = get_venv_python_bin(&project.venv_path());
    if !python.is_file() {
        warn!("Project is not synced, no virtualenv found. Run `rye sync`.");
        return Ok(());
    }
    let uv = UvBuilder::new()
        .with_output(CommandOutput::Normal)
        .ensure_exists()?;
    uv.venv(
        &project.venv_path(),
        &python,
        &project.venv_python_version()?,
        None,
    )?
    .freeze()?;
    Ok(())
}
