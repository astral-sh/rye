use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use anyhow::Error;
use clap::Parser;

use crate::bootstrap::ensure_self_venv;
use crate::consts::VENV_BIN;
use crate::pyproject::{locate_projects, PyProject};
use crate::utils::{CommandOutput, QuietExit};

#[derive(Parser, Debug)]
pub struct PyTestArgs {
    /// List of files or directories to limit the operation to
    paths: Vec<PathBuf>,
    /// Perform the operation on all packages
    #[arg(short, long)]
    all: bool,
    /// Perform the operation on a specific package
    #[arg(short, long)]
    package: Vec<String>,
    /// Use this pyproject.toml file
    #[arg(long, value_name = "PYPROJECT_TOML")]
    pyproject: Option<PathBuf>,
    /// Enables verbose diagnostics.
    #[arg(short, long)]
    verbose: bool,
    /// Turns off all output.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
    /// Extra arguments to ruff
    #[arg(last = true)]
    extra_args: Vec<OsString>,
}

pub fn execute_pytest(args: PyTestArgs, extra_args: &[&str]) -> Result<(), Error> {
    let project = PyProject::load_or_discover(args.pyproject.as_deref())?;
    let output = CommandOutput::from_quiet_and_verbose(args.quiet, args.verbose);
    let venv = ensure_self_venv(output)?;
    let pytest = venv.join(VENV_BIN).join("pytest");

    let mut pytest_cmd = Command::new(pytest);

    match output {
        CommandOutput::Normal => {}
        CommandOutput::Verbose => {
            pytest_cmd.arg("--verbose");
        }
        CommandOutput::Quiet => {
            pytest_cmd.arg("-q");
        }
    }
    pytest_cmd.args(extra_args);
    pytest_cmd.args(args.extra_args);

    pytest_cmd.arg("--");
    if args.paths.is_empty() {
        let projects = locate_projects(project, args.all, &args.package[..])?;
        for project in projects {
            pytest_cmd.arg(project.root_path().as_os_str());
        }
    } else {
        for file in args.paths {
            pytest_cmd.arg(file.as_os_str());
        }
    }

    let status = pytest_cmd.status()?;
    if !status.success() {
        let code = status.code().unwrap_or(1);
        Err(QuietExit(code).into())
    } else {
        Ok(())
    }
}
