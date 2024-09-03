use std::path::PathBuf;

use anyhow::Error;
use clap::Parser;

use crate::pyproject::PyProject;
use crate::utils::{get_venv_python_bin, CommandOutput};
use crate::uv::{UvBuilder, Venv};

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
        warn!("Project is not synced, no virtualenv found. Run `rye sync`.");
        return Ok(());
    }
    let uv = UvBuilder::new()
        .with_output(CommandOutput::Normal)
        .ensure_exists()?;
    uv.read_only_venv(&project.venv_path())?.freeze()?;
    Ok(())
}
