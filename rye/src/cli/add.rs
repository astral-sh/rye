use std::process::{Command, Stdio};
use std::str::FromStr;

use anyhow::{bail, Error};
use clap::Parser;
use pep440_rs::VersionSpecifiers;
use pep508_rs::{Requirement, VersionOrUrl};
use serde::Deserialize;

use crate::bootstrap::ensure_self_venv;
use crate::pyproject::{DependencyKind, PyProject};
use crate::utils::{format_requirement, CommandOutput};

#[derive(Deserialize, Debug)]
struct Match {
    name: String,
    version: String,
}

/// Adds a Python package to this project.
#[derive(Parser, Debug)]
pub struct Args {
    /// The package to add as PEP 508 requirement string. e.g. 'flask==2.2.3'
    requirements: Vec<String>,
    /// Add this as dev dependency.
    #[arg(long)]
    dev: bool,
    /// Add this to an optional dependency group.
    #[arg(long, conflicts_with = "dev")]
    optional: Option<String>,
    /// Adds a dependency with a specific feature.
    #[arg(short, long)]
    features: Vec<String>,
    /// Enables verbose diagnostics.
    #[arg(short, long)]
    verbose: bool,
    /// Turns off all output.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);
    let mut unearth_path = ensure_self_venv(output)?;
    let mut added = Vec::new();
    unearth_path.push("bin");
    unearth_path.push("unearth");

    let mut pyproject_toml = PyProject::discover()?;

    for str_requirement in cmd.requirements {
        let mut requirement = Requirement::from_str(&str_requirement)?;
        for feature in cmd.features.iter().flat_map(|x| x.split(',')) {
            let feature = feature.trim();
            let extras = requirement.extras.get_or_insert_with(|| Vec::new());
            if !extras.iter().any(|x| x == feature) {
                extras.push(feature.into());
            }
        }

        let unearth = Command::new(&unearth_path)
            .arg("--")
            .arg(&str_requirement)
            .stdout(Stdio::piped())
            .output()?;
        if !unearth.status.success() {
            bail!("did not find package {}", format_requirement(&requirement));
        }

        let m: Match = serde_json::from_slice(&unearth.stdout)?;
        if requirement.version_or_url.is_none() {
            requirement.version_or_url = Some(VersionOrUrl::VersionSpecifier(
                VersionSpecifiers::from_str(&format!("~={}", m.version))?,
            ));
        }
        requirement.name = m.name;

        pyproject_toml.add_dependency(
            &requirement,
            if cmd.dev {
                DependencyKind::Dev
            } else if let Some(ref section) = cmd.optional {
                DependencyKind::Optional(section.into())
            } else {
                DependencyKind::Normal
            },
        )?;
        added.push(requirement);
    }

    pyproject_toml.save()?;

    if output != CommandOutput::Quiet {
        for ref requirement in added {
            println!("Added {}", format_requirement(&requirement));
        }
    }

    Ok(())
}
