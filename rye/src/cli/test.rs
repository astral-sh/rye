use anyhow::Error;
use clap::Parser;

use crate::utils::pytest;

/// Run the code formatter on the project.
///
/// This invokes ruff in format mode.
#[derive(Parser, Debug)]
pub struct Args {
    #[command(flatten)]
    pytest: pytest::PyTestArgs,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let args: &[&str] = &[];
    pytest::execute_pytest(cmd.pytest, args)
}
