use anyhow::{anyhow, Context, Error};
use clap::Parser;

use crate::bootstrap::fetch;
use crate::platform::get_python_version_request_from_pyenv_pin;
use crate::pyproject::PyProject;
use crate::sources::PythonVersionRequest;
use crate::utils::CommandOutput;

/// Fetches a Python interpreter for the local machine.
#[derive(Parser, Debug)]
pub struct Args {
    /// The version of Python to fetch.
    ///
    /// If no version is provided, the requested version will be fetched.
    version: Option<String>,
    /// Overrides the architecture to fetch.
    ///
    /// When a non native architecture is fetched, the toolchain is
    /// installed under an alias.
    arch: Option<String>,
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
                get_python_version_request_from_pyenv_pin(&std::env::current_dir()?).ok_or_else(
                    || anyhow!("not sure what to fetch, please provide an explicit version"),
                )?
            }
        }
    };

    fetch(&version, output).context("error while fetching python installation")?;
    Ok(())
}
