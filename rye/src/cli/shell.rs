use std::env;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Error};
use clap::Parser;
use console::style;

use crate::pyproject::PyProject;
use crate::sync::{sync, SyncOptions};
use crate::utils::QuietExit;

/// Spawns a shell with the virtualenv activated.
#[derive(Parser, Debug)]
pub struct Args {
    /// Do not show banner
    #[arg(long)]
    no_banner: bool,
    /// Allow nested invocations.
    #[arg(long)]
    allow_nested: bool,
    /// Use this pyproject.toml file
    #[arg(long, value_name = "PYPROJECT_TOML")]
    pyproject: Option<PathBuf>,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    if !cmd.allow_nested && env::var("__RYE_SHELL").ok().as_deref() == Some("1") {
        bail!("cannot invoke recursive rye shell");
    }

    let pyproject = PyProject::load_or_discover(cmd.pyproject.as_deref())?;
    sync(SyncOptions::python_only().pyproject(cmd.pyproject))
        .context("failed to sync ahead of shell")?;

    let venv_path = pyproject.venv_path();
    let venv_bin = venv_path.join("bin");

    let mut shell = Command::new(env::var("SHELL")?);
    shell.arg("-l").env("VIRTUAL_ENV", &*venv_path);

    if let Some(path) = env::var_os("PATH") {
        let mut new_path = venv_bin.as_os_str().to_owned();
        new_path.push(":");
        new_path.push(path);
        shell.env("PATH", new_path);
    } else {
        shell.env("PATH", &*venv_bin);
    }
    shell.env_remove("PYTHONHOME");
    shell.env("__RYE_SHELL", "1");

    if !cmd.no_banner {
        eprintln!(
            "Spawning virtualenv shell from {}",
            style(&venv_path.display()).cyan()
        );
        eprintln!("Leave shell with 'exit'");
    }

    let status = shell.status()?;
    if !status.success() {
        let code = status.code().unwrap_or(1);
        Err(QuietExit(code).into())
    } else {
        Ok(())
    }
}
