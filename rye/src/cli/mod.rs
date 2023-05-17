use std::env;

use anyhow::Error;
use clap::Parser;

mod add;
mod build;
mod fetch;
mod init;
mod install;
mod lock;
mod make_req;
mod pin;
mod publish;
mod remove;
mod run;
mod rye;
mod shell;
mod shim;
mod show;
mod sync;
mod toolchain;
mod tools;
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
    MakeReq(make_req::Args),
    Pin(pin::Args),
    Publish(publish::Args),
    Remove(remove::Args),
    Run(run::Args),
    Shell(shell::Args),
    Show(show::Args),
    Sync(sync::Args),
    Toolchain(toolchain::Args),
    Tools(tools::Args),
    #[command(name = "self")]
    Rye(rye::Args),
    Uninstall(uninstall::Args),
}

pub fn execute() -> Result<(), Error> {
    // common initialization
    crate::platform::init()?;
    crate::config::load()?;

    let args = env::args_os().collect::<Vec<_>>();

    // if we're shimmed, execute the shim.  This won't return.
    shim::execute_shim(&args)?;

    // special case for self installation
    if args.len() == 1 && rye::auto_self_install()? {
        return Ok(());
    }

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
        Command::MakeReq(cmd) => make_req::execute(cmd),
        Command::Pin(cmd) => pin::execute(cmd),
        Command::Publish(cmd) => publish::execute(cmd),
        Command::Remove(cmd) => remove::execute(cmd),
        Command::Run(cmd) => run::execute(cmd),
        Command::Shell(cmd) => shell::execute(cmd),
        Command::Show(cmd) => show::execute(cmd),
        Command::Sync(cmd) => sync::execute(cmd),
        Command::Toolchain(cmd) => toolchain::execute(cmd),
        Command::Tools(cmd) => tools::execute(cmd),
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
