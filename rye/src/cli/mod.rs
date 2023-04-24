use anyhow::Error;
use clap::Parser;

mod add;
mod fetch;
mod init;
mod install;
mod lock;
mod pin;
mod remove;
mod run;
mod shim;
mod show;
mod sync;
mod toolchain;
mod uninstall;

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    Add(add::Args),
    Fetch(fetch::Args),
    Init(init::Args),
    Install(install::Args),
    Lock(lock::Args),
    Pin(pin::Args),
    Remove(remove::Args),
    Run(run::Args),
    Show(show::Args),
    Sync(sync::Args),
    Toolchain(toolchain::Args),
    Uninstall(uninstall::Args),
}

pub fn execute() -> Result<(), Error> {
    // if we're shimmed, execute the shim.  This won't return.
    shim::execute_shim()?;

    let args = Args::parse();

    match args.command {
        Command::Add(cmd) => add::execute(cmd),
        Command::Fetch(cmd) => fetch::execute(cmd),
        Command::Init(cmd) => init::execute(cmd),
        Command::Install(cmd) => install::execute(cmd),
        Command::Lock(cmd) => lock::execute(cmd),
        Command::Pin(cmd) => pin::execute(cmd),
        Command::Remove(cmd) => remove::execute(cmd),
        Command::Run(cmd) => run::execute(cmd),
        Command::Show(cmd) => show::execute(cmd),
        Command::Sync(cmd) => sync::execute(cmd),
        Command::Toolchain(cmd) => toolchain::execute(cmd),
        Command::Uninstall(cmd) => uninstall::execute(cmd),
    }
}
