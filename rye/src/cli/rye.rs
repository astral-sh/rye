use std::process::Command;

use anyhow::{bail, Context, Error};
use clap::Parser;

/// Rye self management
#[derive(Parser, Debug)]
pub struct Args {
    #[command(subcommand)]
    command: SubCommand,
}

/// Performs an update of rye.
///
/// This currently just is an alias to running cargo install again with the
/// right arguments.
#[derive(Parser, Debug)]
pub struct UpdateCommand {
    /// Update to a specific tag.
    #[arg(long)]
    tag: Option<String>,
    /// Update to a specific git rev.
    #[arg(long, conflicts_with = "tag")]
    rev: Option<String>,
    /// Force reinstallation
    #[arg(long)]
    force: bool,
}

#[derive(Parser, Debug)]
enum SubCommand {
    Update(UpdateCommand),
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    match cmd.command {
        SubCommand::Update(args) => update(args),
    }
}

fn update(args: UpdateCommand) -> Result<(), Error> {
    let mut cmd = Command::new("cargo");
    cmd.arg("install")
        .arg("--git")
        .arg("https://github.com/mitsuhiko/rye");
    if let Some(ref rev) = args.rev {
        cmd.arg("--rev");
        cmd.arg(rev);
    } else if let Some(ref tag) = args.tag {
        cmd.arg("--tag");
        cmd.arg(tag);
    }
    if args.force {
        cmd.arg("--force");
    }
    cmd.arg("rye");
    let status = cmd.status().context("unable to update via cargo-install")?;
    if !status.success() {
        bail!("failed to self-update via cargo-install");
    }

    Ok(())
}
