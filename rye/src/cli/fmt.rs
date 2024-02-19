use anyhow::Error;
use clap::Parser;

use crate::utils::ruff;

/// Run the code formatter on the project.
///
/// This invokes ruff in format mode.
#[derive(Parser, Debug)]
pub struct Args {
    #[command(flatten)]
    ruff: ruff::RuffArgs,
    /// Run format in check mode
    #[arg(long)]
    check: bool,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let mut args = Vec::new();
    args.push("format");
    if cmd.check {
        args.push("--check");
    }
    ruff::execute_ruff(cmd.ruff, &args)
}
