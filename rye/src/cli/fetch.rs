use anyhow::{Context, Error};
use clap::Parser;

use crate::bootstrap::fetch;
use crate::utils::CommandOutput;

/// Fetches a Python interpreter for the local machine.
#[derive(Parser, Debug)]
pub struct Args {
    /// The version of Python to fetch.
    version: String,
    /// Overrides the architecture to fetch.
    ///
    /// When a non native architecture is fetched, the toolchain is
    /// installed under an alias.
    arch: Option<String>,
    /// Enables verbose diagnostics.
    #[arg(short, long)]
    verbose: bool,
    /// Turns off all output.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);
    fetch(&cmd.version.parse()?, output).context("error while fetching python installation")?;
    Ok(())
}
