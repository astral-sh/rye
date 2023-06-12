use std::path::PathBuf;

use anyhow::Error;
use clap::Parser;

use crate::lock::LockOptions;
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
    /// Update a specific package.
    #[arg(long)]
    update: Vec<String>,
    /// Update all packages to the latest
    #[arg(long)]
    update_all: bool,
    /// Update to pre-release versions
    #[arg(long)]
    pre: bool,
    /// Extras/features to enable when locking the workspace.
    #[arg(long)]
    features: Vec<String>,
    /// Enables all features.
    #[arg(long)]
    all_features: bool,
    /// Use this pyproject.toml file
    #[arg(long, value_name = "PYPROJECT_TOML")]
    pyproject: Option<PathBuf>,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);
    sync(SyncOptions {
        output,
        mode: SyncMode::LockOnly,
        lock_options: LockOptions {
            update: cmd.update,
            update_all: cmd.update_all,
            pre: cmd.pre,
            features: cmd.features,
            all_features: cmd.all_features,
        },
        pyproject: cmd.pyproject,
        ..SyncOptions::default()
    })?;
    Ok(())
}
