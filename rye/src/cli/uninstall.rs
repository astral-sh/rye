use std::path::Path;
use anyhow::{anyhow, Error};
use clap::Parser;

use crate::installer::uninstall;
use crate::pyproject::PyProject;
use crate::utils::CommandOutput;

/// Uninstalls a global tool.
#[derive(Parser, Debug)]
pub struct Args {
    /// The package to uninstall
    #[arg(default_value = ".")]
    name: String,
    /// Enables verbose diagnostics.
    #[arg(short, long)]
    verbose: bool,
    /// Turns off all output.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);

    let name = if cmd.name == "." {
        get_project_name_in_current_directory().ok().unwrap()
    } else {
        cmd.name.to_string()
    };

    uninstall(name.as_str(), output)?;
    Ok(())
}

pub fn get_project_name_in_current_directory() -> Result<String, Error> {
    Ok(PyProject::load(Path::new("pyproject.toml"))?
        .name()
        .ok_or_else(|| anyhow!("project name not found"))?
        .to_string())
}
