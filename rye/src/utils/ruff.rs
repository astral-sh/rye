use std::env;
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
pub struct RuffArgs {
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

pub fn execute_ruff(args: RuffArgs, extra_args: &[&str]) -> Result<(), Error> {
    let project = PyProject::load_or_discover(args.pyproject.as_deref())?;
    let output = CommandOutput::from_quiet_and_verbose(args.quiet, args.verbose);
    let venv = ensure_self_venv(output)?;
    let ruff = venv.join(VENV_BIN).join("ruff");

    let mut ruff_cmd = Command::new(ruff);
    if env::var_os("RUFF_CACHE_DIR").is_none() {
        ruff_cmd.env(
            "RUFF_CACHE_DIR",
            project.workspace_path().join(".ruff_cache"),
        );
    }
    match output {
        CommandOutput::Normal => {}
        CommandOutput::Verbose => {
            ruff_cmd.arg("--verbose");
        }
        CommandOutput::Quiet => {
            ruff_cmd.arg("-q");
        }
    }
    ruff_cmd.args(extra_args);
    ruff_cmd.args(args.extra_args);

    ruff_cmd.arg("--");
    if args.paths.is_empty() {
        let projects = locate_projects(project, args.all, &args.package[..])?;
        for project in projects {
            ruff_cmd.arg(project.root_path().as_os_str());
        }
    } else {
        for file in args.paths {
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
