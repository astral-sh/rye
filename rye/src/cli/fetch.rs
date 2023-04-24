use anyhow::{Context, Error};
use clap::Parser;

use crate::bootstrap::fetch;
use crate::utils::CommandOutput;

/// Fetches a Python interpreter for the local machine.
#[derive(Parser, Debug)]
pub struct Args {
    /// The version of Python to fetch.
    version: String,
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
