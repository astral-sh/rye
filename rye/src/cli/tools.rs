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
    /// Show all the scripts installed by the tools.
    #[arg(short = 's', long)]
    include_scripts: bool,
    /// Show the version of tools.
    #[arg(short = 'v', long)]
    include_version: bool,
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
    tools.sort_by_key(|(tool, _)| tool.clone());

    for (tool, mut info) in tools {
        if !info.valid {
            echo!("{} ({})", style(tool).red(), style("seems broken").red());
            continue;
        }
        if cmd.include_version {
            if let Some(ref venv) = info.venv_marker {
                echo!("{} {} ({})", style(tool).cyan(), info.version, venv.python);
            } else {
                echo!("{} {}", style(tool).cyan(), info.version);
            }
        } else {
            echo!("{}", style(tool).cyan());
        }
        if cmd.include_scripts {
            info.scripts.sort();
            for script in info.scripts {
                echo!("  {}", script);
            }
        }
    }

    Ok(())
}
