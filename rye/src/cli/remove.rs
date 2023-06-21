use std::str::FromStr;

use anyhow::Error;
use clap::Parser;
use pep508_rs::Requirement;

use crate::pyproject::{DependencyKind, PyProject};
use crate::utils::{format_requirement, CommandOutput};

/// Removes a package from this project.
#[derive(Parser, Debug)]
pub struct Args {
    /// The packages to remove.
    requirements: Vec<String>,
    /// Remove this from dev dependencies.
    #[arg(long)]
    dev: bool,
    /// Remove this from an optional dependency group.
    #[arg(long, conflicts_with = "dev")]
    optional: Option<String>,
    /// Enables verbose diagnostics.
    #[arg(short, long)]
    verbose: bool,
    /// Turns off all output.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);
    let mut removed_packages = Vec::new();

    let mut pyproject_toml = PyProject::discover()?;
    for str_requirement in cmd.requirements {
        let requirement = Requirement::from_str(&str_requirement)?;
        if let Some(removed) = pyproject_toml.remove_dependency(
            &requirement,
            if cmd.dev {
                DependencyKind::Dev
            } else if let Some(ref section) = cmd.optional {
                DependencyKind::Optional(section.into())
            } else {
                DependencyKind::Normal
            },
        )? {
            removed_packages.push(removed);
        }
    }

    pyproject_toml.save()?;

    if output != CommandOutput::Quiet {
        for requirement in removed_packages {
            echo!("Removed {}", format_requirement(&requirement));
        }
    }

    Ok(())
}
