use anyhow::Error;
use clap::Parser;

use crate::utils::ruff;

/// Run the linter on the project.
///
/// This invokes ruff in lint mode.
#[derive(Parser, Debug)]
pub struct Args {
    #[command(flatten)]
    ruff: ruff::RuffArgs,
    /// Apply fixes.
    #[arg(long)]
    fix: bool,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let mut args = Vec::new();
    args.push("check");
    if cmd.fix {
        args.push("--fix");
    }
    ruff::execute_ruff(cmd.ruff, &args)
}
