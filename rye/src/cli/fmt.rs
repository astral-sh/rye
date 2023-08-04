use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use anyhow::Error;
use clap::Parser;

use crate::bootstrap::ensure_self_venv;
use crate::pyproject::PyProject;
use crate::utils::{get_venv_python_bin, CommandOutput};

#[derive(Parser, Debug)]
pub struct Args {
    /// Do not actually reformat files, only show whether they would be changed.
    #[arg(short, long)]
    check: bool,
    /// Use this pyproject.toml file
    #[arg(long, value_name = "PYPROJECT_TOML")]
    pyproject: Option<PathBuf>,
    /// Enables verbose diagnostics.
    #[arg(short, long)]
    verbose: bool,
    /// Turns off all output.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
    /// Arbitrary extra arguments for black.
    #[arg(value_name = "-- <black_options>")]
    black_args: Vec<OsString>,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);
    let self_venv = ensure_self_venv(output)?;
    let python = get_venv_python_bin(&self_venv);
    let pyproject = PyProject::load_or_discover(cmd.pyproject.as_deref())?;
    // TODO: consider adding an option to format all projects from a workspace.
    let black_run_dir = pyproject.root_path();
    let mut black_cmd = Command::new(python);
    black_cmd.arg("-mblack");
    // Transmit --quiet / --verbose to black also.
    if cmd.quiet {
        black_cmd.arg("--quiet");
    } else if cmd.verbose {
        black_cmd.arg("--verbose");
    }
    // Transmit --check to black
    if cmd.check {
        black_cmd.arg("--check");
    }
    // Transmit arbitrary options to black.
    black_cmd.args(cmd.black_args);
    black_cmd.arg(&*black_run_dir);
    black_cmd.status()?;
    Ok(())
}
