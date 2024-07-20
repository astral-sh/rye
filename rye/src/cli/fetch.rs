use std::path::PathBuf;

use anyhow::{Context, Error};
use clap::Parser;

use crate::bootstrap::{fetch, FetchOptions};
use crate::config::Config;
use crate::platform::get_python_version_request_from_pyenv_pin;
use crate::pyproject::PyProject;
use crate::sources::py::PythonVersionRequest;
use crate::utils::CommandOutput;

/// Fetches a Python interpreter for the local machine.
#[derive(Parser, Debug)]
pub struct Args {
    /// The version of Python to fetch.
    ///
    /// If no version is provided, the requested version from local project or `.python-version` will be fetched.
    version: Option<String>,
    /// Fetch the Python toolchain even if it is already installed.
    #[arg(short, long)]
    force: bool,
    /// Fetches the Python toolchain into an explicit location rather.
    #[arg(long)]
    target_path: Option<PathBuf>,
    /// Fetches with build info.
    #[arg(long)]
    build_info: bool,
    /// Fetches without build info.
    #[arg(long, conflicts_with = "build_info")]
    no_build_info: bool,
    /// Enables verbose diagnostics.
    #[arg(short, long)]
    verbose: bool,
    /// Turns off all output.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);

    let version: PythonVersionRequest = match cmd.version {
        Some(version) => version.parse()?,
        None => {
            if let Ok(pyproject) = PyProject::discover() {
                pyproject.venv_python_version()?.into()
            } else {
                match get_python_version_request_from_pyenv_pin(&std::env::current_dir()?) {
                    Some(version) => version,
                    None => Config::current().default_toolchain()?,
                }
            }
        }
    };

    fetch(
        &version,
        FetchOptions {
            output,
            force: cmd.force,
            target_path: cmd.target_path,
            build_info: if cmd.build_info {
                Some(true)
            } else if cmd.no_build_info {
                Some(false)
            } else {
                None
            },
        },
    )
    .context("error while fetching Python installation")?;
    Ok(())
}
