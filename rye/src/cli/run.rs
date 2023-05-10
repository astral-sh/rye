use std::env::{self, join_paths, split_paths};
use std::ffi::OsString;
use std::process::Command;

use anyhow::{Context, Error};
use clap::Parser;
use console::style;

use crate::pyproject::{PyProject, Script};
use crate::sync::{sync, SyncOptions};
use crate::utils::exec_spawn;

/// Runs a command installed into this package.
#[derive(Parser, Debug)]
#[command(arg_required_else_help(false))]
pub struct Args {
    /// List all commands
    #[arg(short, long)]
    list: bool,
    /// The command to run
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Parser, Debug)]
enum Cmd {
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let pyproject = PyProject::discover()?;

    // make sure we have the minimal virtualenv.
    sync(SyncOptions::python_only()).context("failed to sync ahead of run")?;
    let venv_bin = pyproject.venv_bin_path();

    if cmd.list || cmd.cmd.is_none() {
        return list_scripts(&pyproject);
    }
    let mut args = match cmd.cmd {
        Some(Cmd::External(args)) => args,
        None => unreachable!(),
    };

    // do we have a custom script to invoke?
    match pyproject.get_script_cmd(&args[0].to_string_lossy()) {
        Some(Script::Cmd(script_args)) if !script_args.is_empty() => {
            let script_target = venv_bin.join(&script_args[0]);
            if script_target.is_file() {
                args = Some(script_target.as_os_str().to_owned())
                    .into_iter()
                    .chain(script_args.into_iter().map(OsString::from).skip(1))
                    .chain(args.into_iter().skip(1))
                    .collect();
            } else {
                args = script_args
                    .into_iter()
                    .map(OsString::from)
                    .chain(args.into_iter().skip(1))
                    .collect();
            }
        }
        Some(Script::External(_)) => {
            args[0] = venv_bin.join(&args[0]).into();
        }
        _ => {}
    }

    let mut cmd = Command::new(&args[0]);
    cmd.args(&args[1..]);

    // when we spawn into a script, we implicitly activate the virtualenv to make
    // the life of tools easier that expect to be in one.
    env::set_var("VIRTUAL_ENV", &*pyproject.venv_path());
    if let Some(path) = env::var_os("PATH") {
        let mut paths = split_paths(&path).collect::<Vec<_>>();
        paths.insert(0, venv_bin.into());
        let new_path = join_paths(paths)?;
        env::set_var("PATH", new_path);
    } else {
        env::set_var("PATH", &*venv_bin);
    }
    env::remove_var("PYTHONHOME");

    match exec_spawn(&mut cmd)? {};
}

fn list_scripts(pyproject: &PyProject) -> Result<(), Error> {
    let mut scripts: Vec<_> = pyproject
        .list_scripts()
        .into_iter()
        .filter_map(|name| {
            let script = pyproject.get_script_cmd(&name)?;
            Some((name, script))
        })
        .collect();
    scripts.sort_by(|a, b| a.0.to_ascii_lowercase().cmp(&b.0.to_ascii_lowercase()));
    for (name, script) in scripts {
        if matches!(script, Script::External(_)) {
            println!("{}", name);
        } else {
            println!("{} ({})", name, style(script).dim());
        }
    }
    Ok(())
}
