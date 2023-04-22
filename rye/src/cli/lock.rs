use anyhow::Error;
use clap::Parser;

use crate::sync::{sync, SyncMode, SyncOptions};
use crate::utils::CommandOutput;

/// Updates the lockfiles without installing dependencies.
#[derive(Parser, Debug)]
pub struct Args {
    /// Enables verbose diagnostics.
    #[arg(short, long)]
    verbose: bool,
    /// Turns off all output.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
    /// Upgrade all packages to the latest
    #[arg(long)]
    upgrade_all: bool,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);
    sync(SyncOptions {
        output,
        mode: SyncMode::LockOnly,
        upgrade_all: cmd.upgrade_all,
        ..SyncOptions::default()
    })?;
    Ok(())
}
