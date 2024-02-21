use std::env;

use anyhow::{bail, Error};
use clap::Parser;

mod add;
mod build;
mod config;
mod fetch;
mod fmt;
mod init;
mod install;
mod lint;
mod list;
mod lock;
mod make_req;
mod pin;
mod publish;
mod remove;
mod run;
mod rye;
mod shim;
mod show;
mod sync;
mod toolchain;
mod tools;
mod uninstall;
mod version;

use git_testament::git_testament;

use crate::bootstrap::SELF_PYTHON_TARGET_VERSION;
use crate::config::Config;
use crate::platform::symlinks_supported;

git_testament!(TESTAMENT);

/// An Experimental Package Management Solution for Python
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
    Config(config::Args),
    Fetch(fetch::Args),
    #[command(alias = "format")]
    Fmt(fmt::Args),
    Init(init::Args),
    Install(install::Args),
    Lock(lock::Args),
    #[command(alias = "check")]
    Lint(lint::Args),
    MakeReq(make_req::Args),
    Pin(pin::Args),
    Publish(publish::Args),
    Remove(remove::Args),
    Run(run::Args),
    Show(show::Args),
    Sync(sync::Args),
    Toolchain(toolchain::Args),
    Tools(tools::Args),
    #[command(name = "self")]
    Rye(rye::Args),
    Uninstall(uninstall::Args),
    Version(version::Args),
    List(list::Args),
    #[command(hide = true)]
    Shell(shell::Args),
}

pub mod shell {
    /// The shell command was removed.
    #[derive(clap::Parser, Debug)]
    pub struct Args {}
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

    let args = Args::try_parse()?;
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
        Command::Config(cmd) => config::execute(cmd),
        Command::Fetch(cmd) => fetch::execute(cmd),
        Command::Fmt(cmd) => fmt::execute(cmd),
        Command::Init(cmd) => init::execute(cmd),
        Command::Install(cmd) => install::execute(cmd),
        Command::Lock(cmd) => lock::execute(cmd),
        Command::Lint(cmd) => lint::execute(cmd),
        Command::MakeReq(cmd) => make_req::execute(cmd),
        Command::Pin(cmd) => pin::execute(cmd),
        Command::Publish(cmd) => publish::execute(cmd),
        Command::Remove(cmd) => remove::execute(cmd),
        Command::Run(cmd) => run::execute(cmd),
        Command::Show(cmd) => show::execute(cmd),
        Command::Sync(cmd) => sync::execute(cmd),
        Command::Toolchain(cmd) => toolchain::execute(cmd),
        Command::Tools(cmd) => tools::execute(cmd),
        Command::Rye(cmd) => rye::execute(cmd),
        Command::Uninstall(cmd) => uninstall::execute(cmd),
        Command::Version(cmd) => version::execute(cmd),
        Command::List(cmd) => list::execute(cmd),
        Command::Shell(..) => {
            bail!(
                "unknown command. The shell command was removed. Activate the virtualenv with '{}' instead.",
                if cfg!(windows) {
                    ".venv\\Scripts\\activate"
                } else {
                    ". .venv/bin/activate"
                }
            );
        }
    }
}

fn print_version() -> Result<(), Error> {
    echo!("rye {}", env!("CARGO_PKG_VERSION"));
    echo!("commit: {}", TESTAMENT.commit);
    echo!(
        "platform: {} ({})",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    echo!("self-python: {}", SELF_PYTHON_TARGET_VERSION);
    echo!("symlink support: {}", symlinks_supported());
    echo!("uv enabled: {}", Config::current().use_uv());
    Ok(())
}
