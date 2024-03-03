use std::path::PathBuf;

use anyhow::Error;
use clap::Parser;

use crate::utils::pytest;

/// Run the tests on the project.
#[derive(Parser, Debug)]
pub struct Args {
    #[command(flatten)]
    pytest: pytest::PyTestArgs,

    // Ignores the specified path
    #[arg(short, long)]
    ignore: Vec<PathBuf>,

    // Disable test output capture to stdout
    #[arg(long, name = "no-capture")]
    no_capture: bool,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let mut args = Vec::new();
    args.push("--ignore=target".to_string()); // ignores the cargo `target` directory by default
    if cmd.no_capture {
        args.push("-s".to_string());
    }
    args.extend(
        cmd.ignore
            .iter()
            .map(|p| format!("--ignore={}", p.to_string_lossy())),
    );
    pytest::execute_pytest(cmd.pytest, &args)
}
