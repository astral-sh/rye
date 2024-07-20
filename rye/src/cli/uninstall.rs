use anyhow::Error;
use clap::Parser;

use crate::installer::uninstall;
use crate::utils::CommandOutput;

/// Uninstalls a global tool.
#[derive(Parser, Debug)]
pub struct Args {
    /// The package to uninstall.
    name: String,
    /// Enables verbose diagnostics.
    #[arg(short, long)]
    verbose: bool,
    /// Turns off all output.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);
    uninstall(&cmd.name, output)?;
    Ok(())
}
