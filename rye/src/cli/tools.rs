use anyhow::Error;
use clap::Parser;
use console::style;

use crate::installer::list_installed_tools;

/// Helper utility to manage global tools.
#[derive(Parser, Debug)]
pub struct Args {
    #[command(subcommand)]
    command: SubCommand,
}

/// List all registered tools
#[derive(Parser, Debug)]
pub struct ListCommand {
    /// Also how all the scripts installed by the tools.
    #[arg(short, long)]
    include_scripts: bool,
}

#[derive(Parser, Debug)]
#[allow(clippy::large_enum_variant)]
enum SubCommand {
    Install(crate::cli::install::Args),
    Uninstall(crate::cli::uninstall::Args),
    List(ListCommand),
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    match cmd.command {
        SubCommand::Install(args) => crate::cli::install::execute(args),
        SubCommand::Uninstall(args) => crate::cli::uninstall::execute(args),
        SubCommand::List(args) => list_tools(args),
    }
}

fn list_tools(cmd: ListCommand) -> Result<(), Error> {
    let mut tools = list_installed_tools()?.into_iter().collect::<Vec<_>>();
    tools.sort();

    for (tool, mut scripts) in tools {
        echo!("{}", style(tool).cyan());
        if cmd.include_scripts {
            scripts.sort();
            for script in scripts {
                echo!("  {}", script);
            }
        }
    }

    Ok(())
}
