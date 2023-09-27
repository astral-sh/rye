use std::str::FromStr;

use crate::pyproject::PyProject;
use anyhow::{anyhow, bail, Error};
use clap::{Parser, ValueEnum};
use pep440_rs::Version;

/// Get or set project version
#[derive(Parser, Debug)]
pub struct Args {
    /// The version to set
    version: Option<String>,
    /// The version bump to apply
    #[arg(short, long)]
    bump: Option<Bump>,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Bump {
    Major,
    Minor,
    Patch,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let mut pyproject_toml = PyProject::discover()?;
    match cmd.version {
        Some(version) => {
            let version =
                Version::from_str(&version).map_err(|msg| anyhow!("invalid version: {}", msg))?;
            if pyproject_toml
                .dynamic()
                .unwrap()
                .contains(&"version".to_string())
            {
                bail!("unsupported set dynamic version");
            } else {
                pyproject_toml.set_version(&version);
                pyproject_toml.save()?;

                echo!("version set to {}", version);
            }
        }
        None => {
            let mut version = pyproject_toml.version()?;
            match cmd.bump {
                Some(bump) => bump_version(&mut version, bump, &mut pyproject_toml)?,
                None => echo!("{}", version),
            }
        }
    }
    Ok(())
}

fn bump_version(version: &mut Version, bump: Bump, pyproject: &mut PyProject) -> Result<(), Error> {
    if version.is_post() {
        version.post = None;
    }
    if version.is_dev() {
        version.dev = None;
        warn!("dev version will be bumped to release version");
    } else {
        let index = bump as usize;
        if version.release.get(index).is_none() {
            version.release.resize(index + 1, 0);
        }
        version.release[index] += 1;
        for i in index + 1..version.release.len() {
            version.release[i] = 0;
        }
    }

    pyproject.set_version(version);
    pyproject.save().unwrap();

    echo!("version bumped to {}", version);

    Ok(())
}
