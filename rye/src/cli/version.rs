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
    /// The version bump to apply
    #[arg(short, long)]
    bump: Option<String>,
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
            let mut version = pyproject_toml.version()?;
            if let Some(bump) = cmd.bump {
                let bumped = bump_version(&mut version, &bump);
                pyproject_toml.set_version(&version);
                pyproject_toml.save()?;
                if bumped {
                    eprintln!("version bumped to {}", version);
                }
            } else {
                eprintln!("{}", version);
            }
        }
    }
    Ok(())
}

fn bump_version(version: &mut Version, bump: &str) -> bool {
    match bump {
        "major" => version.release[0] += 1,
        "minor" => version.release[1] += 1,
        "patch" => version.release[2] += 1,
        _ => return false,
    }
    true
}
