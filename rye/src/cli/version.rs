use std::str::FromStr;

use anyhow::{anyhow, Error};
use clap::Parser;
use pep440_rs::Version;

use crate::pyproject::PyProject;

/// Get or set project version
#[derive(Parser, Debug)]
pub struct Args {
    /// The version to set
    version: Option<String>,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let mut pyproject_toml = PyProject::discover()?;
    match cmd.version {
        Some(version) => {
            let version =
                Version::from_str(&version).map_err(|msg| anyhow!("invalid version: {}", msg))?;
            pyproject_toml.set_version(&version);
            pyproject_toml.save()?;

            eprintln!("version set to {}", version);
        }
        None => {
            eprintln!("{}", pyproject_toml.version()?);
        }
    }
    Ok(())
}
