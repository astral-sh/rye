use anyhow::{Context, Error};
use clap::Parser;

use crate::bootstrap::fetch;
use crate::config::Config;
use crate::platform::get_python_version_request_from_pyenv_pin;
use crate::pyproject::PyProject;
use crate::sources::PythonVersionRequest;
use crate::utils::CommandOutput;

/// Fetches a Python interpreter for the local machine. This is an alias of `rye toolchain fetch`.
#[derive(Parser, Debug)]
pub struct Args {
    /// The version of Python to fetch.
    ///
    /// If no version is provided, the requested version from local project or `.python-version` will be fetched.
    version: Option<String>,
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

    fetch(&version, output).context("error while fetching Python installation")?;
    Ok(())
}
