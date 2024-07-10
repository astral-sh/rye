use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{Context, Error};
use clap::Parser;
use pep508_rs::Requirement;

use crate::cli::add::ReqExtras;
use crate::utils::format_requirement;

/// Builds and prints a PEP 508 requirement string from parts.
#[derive(Parser, Debug)]
pub struct Args {
    /// The package to add as PEP 508 requirement string. e.g. 'flask==2.2.3'
    requirements: Vec<String>,
    #[command(flatten)]
    req_extras: ReqExtras,
    /// Use this virtual environment.
    #[arg(long, value_name = "VENV")]
    venv: Option<PathBuf>,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    for requirement_str in cmd.requirements {
        let mut requirement = Requirement::from_str(&requirement_str)
            .with_context(|| format!("unable to parse requirement '{}'", requirement_str))?;
        cmd.req_extras
            .apply_to_requirement(&mut requirement, cmd.venv.as_ref())?;
        echo!("{}", format_requirement(&requirement));
    }

    Ok(())
}
