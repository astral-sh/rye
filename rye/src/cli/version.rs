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
            let version = pyproject_toml.version()?;
            match cmd.bump {
                Some(bump) => bump_version(version, bump, &mut pyproject_toml)?,
                None => echo!("{}", version),
            }
        }
    }
    Ok(())
}

fn bump_version(mut version: Version, bump: Bump, pyproject: &mut PyProject) -> Result<(), Error> {
    if version.is_post() {
        version = version.with_post(None);
    }
    if version.is_dev() {
        version = version.with_dev(None);
        warn!("dev version will be bumped to release version");
    } else {
        let mut release = version.release().to_vec();
        let index = bump as usize;
        if release.get(index).is_none() {
            release.resize(index + 1, 0);
        }
        release[index] += 1;
        release[index + 1..].fill(0);

        version = version.with_release(release);
    }

    pyproject.set_version(&version);
    pyproject.save().unwrap();

    echo!("version bumped to {}", version);

    Ok(())
}
