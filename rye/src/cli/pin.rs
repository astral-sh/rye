use std::env;
use std::fs;

use anyhow::Context;
use anyhow::{anyhow, Error};
use clap::Parser;

use crate::platform::get_pinnable_version;
use crate::pyproject::DiscoveryUnsuccessful;
use crate::pyproject::PyProject;
use crate::sources::PythonVersionRequest;

/// Pins a Python version to this project.
///
/// This will update the `.python-version` to point to the provided version.
/// Additionally it will update `requires-python` in the `pyproject.toml`
/// if it's lower than the current version.  This can be disabled by passing
/// `--no-update-requires-python`.
#[derive(Parser, Debug)]
pub struct Args {
    /// The version of Python to pin.
    version: String,
    /// Issue a relaxed pin
    #[arg(long)]
    relaxed: bool,
    /// Prevent updating requires-python in the pyproject.toml.
    #[arg(long)]
    no_update_requires_python: bool,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let req: PythonVersionRequest = cmd
        .version
        .parse()
        .with_context(|| format!("'{}' is not a valid version", cmd.version))?;
    let to_write = get_pinnable_version(&req, cmd.relaxed)
        .ok_or_else(|| anyhow!("unsupported/unknown version for this platform"))?;

    let pyproject = match PyProject::discover() {
        Ok(proj) => Some(proj),
        Err(err) => {
            if err.is::<DiscoveryUnsuccessful>() {
                // ok
                None
            } else {
                return Err(err);
            }
        }
    };

    let version_file = match pyproject {
        Some(ref proj) => proj.root_path().join(".python-version"),
        None => env::current_dir()?.join(".python-version"),
    };
    fs::write(&version_file, format!("{}\n", to_write))
        .context("failed to write .python-version file")?;

    if !cmd.no_update_requires_python {
        if let Some(mut pyproject_toml) = pyproject {
            let new_version = to_write.parse::<PythonVersionRequest>()?;
            if let Some(curr_version) = pyproject_toml.target_python_version() {
                if new_version < curr_version {
                    pyproject_toml.set_target_python_version(&new_version);
                    pyproject_toml.save()?;
                }
            }
        }
    }

    eprintln!("pinned {} in {}", to_write, version_file.display());

    Ok(())
}
