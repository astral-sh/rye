use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use anyhow::Error;
use clap::Parser;

use crate::bootstrap::ensure_self_venv;
use crate::consts::VENV_BIN;
use crate::pyproject::{locate_projects, PyProject};
use crate::utils::{CommandOutput, QuietExit};

/// Run the linter on the project.
///
/// This invokes ruff in lint mode.
#[derive(Parser, Debug)]
pub struct Args {
    /// List of files or directories to lint
    paths: Vec<PathBuf>,
    /// Lint all packages
    #[arg(short, long)]
    all: bool,
    /// Lint a specific package
    #[arg(short, long)]
    package: Vec<String>,
    /// Use this pyproject.toml file
    #[arg(long, value_name = "PYPROJECT_TOML")]
    pyproject: Option<PathBuf>,
    /// Apply fixes.
    #[arg(long)]
    fix: bool,
    /// Enables verbose diagnostics.
    #[arg(short, long)]
    verbose: bool,
    /// Turns off all output.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
    /// Extra arguments to the linter
    #[arg(last = true)]
    extra_args: Vec<OsString>,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let project = PyProject::load_or_discover(cmd.pyproject.as_deref())?;
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);
    let venv = ensure_self_venv(output)?;
    let ruff = venv.join(VENV_BIN).join("ruff");

    let mut ruff_cmd = Command::new(ruff);
    ruff_cmd.arg("check");
    match output {
        CommandOutput::Normal => {}
        CommandOutput::Verbose => {
            ruff_cmd.arg("--verbose");
        }
        CommandOutput::Quiet => {
            ruff_cmd.arg("-q");
        }
    }

    if cmd.fix {
        ruff_cmd.arg("--fix");
    }
    ruff_cmd.args(cmd.extra_args);

    ruff_cmd.arg("--");
    if cmd.paths.is_empty() {
        let projects = locate_projects(project, cmd.all, &cmd.package[..])?;
        for project in projects {
            ruff_cmd.arg(project.root_path().as_os_str());
        }
    } else {
        for file in cmd.paths {
            ruff_cmd.arg(file.as_os_str());
        }
    }

    let status = ruff_cmd.status()?;
    if !status.success() {
        let code = status.code().unwrap_or(1);
        Err(QuietExit(code).into())
    } else {
        Ok(())
    }
}
