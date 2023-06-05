use std::str::FromStr;

use crate::pyproject::PyProject;
use anyhow::{anyhow, Error};
use clap::Parser;
use console::style;
use pep440_rs::Version;

/// Get or set project version
#[derive(Parser, Debug)]
pub struct Args {
    /// The version to set
    version: Option<String>,
    /// The version bump to apply [major, minor, patch]
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
            match cmd.bump {
                Some(bump) => bump_version(&mut version, &bump, &mut pyproject_toml),
                None => eprintln!("{}", version),
            }
        }
    }
    Ok(())
}

fn bump_version(version: &mut Version, bump: &str, pyproject: &mut PyProject) {
    if version.is_post() {
        version.post = None;
    }
    if version.is_dev() {
        version.dev = None;
        eprintln!(
            "{} dev version will be bumped to release version",
            style("warning:").red()
        );
    } else {
        match bump {
            "major" => version.release[0] += 1,
            "minor" => version.release[1] += 1,
            "patch" => version.release[2] += 1,
            _ => return,
        }
    }

    pyproject.set_version(&version);
    pyproject.save().unwrap();

    eprintln!("version bumped to {}", version);
}
