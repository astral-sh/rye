use std::env;
use std::fs;

use anyhow::Context;
use anyhow::{anyhow, Error};
use clap::Parser;

use crate::config::get_pinnable_version;
use crate::pyproject::PyProject;
use crate::sources::PythonVersionRequest;

/// Pins a Python version to this project.
#[derive(Parser, Debug)]
pub struct Args {
    /// The version of Python to pin.
    version: String,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let req: PythonVersionRequest = cmd.version.parse()?;
    let to_write = get_pinnable_version(&req)
        .ok_or_else(|| anyhow!("unsupported/unknown version for this platform"))?;

    let version_file = match PyProject::discover() {
        Ok(proj) => proj.root_path().join(".python-version"),
        Err(_) => env::current_dir()?.join(".python-version"),
    };
    fs::write(&version_file, format!("{}\n", to_write))
        .context("failed to write .python-version file")?;

    let mut pyproject_toml = PyProject::discover()?;
    let new_version = to_write.parse::<PythonVersionRequest>()?;
    if let Some(curr_version) = pyproject_toml.target_python_version() {
        if new_version < curr_version {
            pyproject_toml.set_target_python_version(&new_version);
            pyproject_toml.save()?;
        }
    }

    eprintln!("pinned {} in {}", to_write, version_file.display());

    Ok(())
}
