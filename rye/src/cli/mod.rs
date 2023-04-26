use anyhow::Error;
use clap::Parser;

mod add;
mod build;
mod fetch;
mod init;
mod install;
mod lock;
mod pin;
mod remove;
mod run;
mod rye;
mod shim;
mod show;
mod sync;
mod toolchain;
mod uninstall;

use git_testament::git_testament;

use crate::bootstrap::SELF_PYTHON_VERSION;

git_testament!(TESTAMENT);

#[derive(Parser, Debug)]
#[command(arg_required_else_help = true)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
    /// Print the version
    #[arg(long)]
    version: bool,
}

#[derive(Parser, Debug)]
enum Command {
    Add(add::Args),
    Build(build::Args),
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
    #[command(name = "self")]
    Rye(rye::Args),
    Uninstall(uninstall::Args),
}

pub fn execute() -> Result<(), Error> {
    // if we're shimmed, execute the shim.  This won't return.
    shim::execute_shim()?;

    let args = Args::parse();
    let cmd = if args.version {
        return print_version();
    } else if let Some(cmd) = args.command {
        cmd
    } else {
        unreachable!()
    };

    match cmd {
        Command::Add(cmd) => add::execute(cmd),
        Command::Build(cmd) => build::execute(cmd),
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
        Command::Rye(cmd) => rye::execute(cmd),
        Command::Uninstall(cmd) => uninstall::execute(cmd),
    }
}

fn print_version() -> Result<(), Error> {
    eprintln!("rye {}", env!("CARGO_PKG_VERSION"));
    eprintln!("commit: {}", TESTAMENT.commit);
    eprintln!(
        "platform: {} ({})",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    eprintln!("self-python: {}", SELF_PYTHON_VERSION);
    Ok(())
}
