use anyhow::Error;
use clap::Parser;

use crate::sync::{sync, SyncMode, SyncOptions};
use crate::utils::CommandOutput;

/// Updates the virtualenv based on the pyproject.toml
#[derive(Parser, Debug)]
pub struct Args {
    /// Force the environment to be re-created
    #[arg(short, long)]
    force: bool,
    /// Do not include dev dependencies.
    #[arg(long)]
    no_dev: bool,
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
        dev: !cmd.no_dev,
        mode: if cmd.force {
            SyncMode::Full
        } else {
            SyncMode::Regular
        },
        force: cmd.force,
        upgrade_all: cmd.upgrade_all,
    })?;
    Ok(())
}
