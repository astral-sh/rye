use std::env::{self, join_paths, split_paths};
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

use anyhow::{bail, Context, Error};
use clap::Parser;
use console::style;

use crate::pyproject::{PyProject, Script};
use crate::sync::{sync, SyncOptions};
use crate::tui::redirect_to_stderr;
use crate::utils::{exec_spawn, get_venv_python_bin, success_status};

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
    /// Use this pyproject.toml file
    #[arg(long, value_name = "PYPROJECT_TOML")]
    pyproject: Option<PathBuf>,
}

#[derive(Parser, Debug)]
enum Cmd {
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let _guard = redirect_to_stderr(true);
    let pyproject = PyProject::load_or_discover(cmd.pyproject.as_deref())?;

    // make sure we have the minimal virtualenv.
    sync(SyncOptions::python_only().pyproject(cmd.pyproject))
        .context("failed to sync ahead of run")?;

    if cmd.list || cmd.cmd.is_none() {
        return list_scripts(&pyproject);
    }
    let args = match cmd.cmd {
        Some(Cmd::External(args)) => args,
        None => unreachable!(),
    };

    invoke_script(&pyproject, args, true)?;
    unreachable!();
}

fn invoke_script(
    pyproject: &PyProject,
    mut args: Vec<OsString>,
    exec: bool,
) -> Result<ExitStatus, Error> {
    let venv_bin = pyproject.venv_bin_path();
    let mut env_overrides = None;

    match pyproject.get_script_cmd(&args[0].to_string_lossy()) {
        Some(Script::Call(entry, env_vars)) => {
            let py = OsString::from(get_venv_python_bin(&pyproject.venv_path()));
            env_overrides = Some(env_vars);
            args = if let Some((module, func)) = entry.split_once(':') {
                if module.is_empty() || func.is_empty() {
                    bail!("Python callable must be in the form <module_name>:<callable_name> or <module_name>")
                }
                let call = if !func.contains('(') {
                    format!("{func}()")
                } else {
                    func.to_string()
                };
                [
                    py,
                    OsString::from("-c"),
                    OsString::from(format!("import sys, {module} as _1; sys.exit(_1.{call})")),
                ]
            } else {
                [py, OsString::from("-m"), OsString::from(entry)]
            }
            .into_iter()
            .chain(args.into_iter().skip(1))
            .collect();
        }
        Some(Script::Cmd(script_args, env_vars)) => {
            if script_args.is_empty() {
                bail!("script has no arguments");
            }
            env_overrides = Some(env_vars);
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
        Some(Script::Chain(commands)) => {
            if args.len() != 1 {
                bail!("extra arguments to chained commands are not allowed");
            }
            for args in commands {
                let status =
                    invoke_script(pyproject, args.into_iter().map(Into::into).collect(), false)?;
                if !status.success() {
                    if !exec {
                        return Ok(status);
                    } else {
                        bail!("script failed with {}", status);
                    }
                }
            }
            if exec {
                std::process::exit(0);
            }
            return Ok(success_status());
        }
        None => {
            bail!("invalid or unknown script '{}'", args[0].to_string_lossy());
        }
    }

    let mut cmd = Command::new(&args[0]);
    cmd.args(&args[1..]);
    cmd.env("VIRTUAL_ENV", &*pyproject.venv_path());
    if let Some(path) = env::var_os("PATH") {
        let mut paths = split_paths(&path).collect::<Vec<_>>();
        paths.insert(0, venv_bin.into());
        let new_path = join_paths(paths)?;
        cmd.env("PATH", new_path);
    } else {
        cmd.env("PATH", &*venv_bin);
    }
    if let Some(env_overrides) = env_overrides {
        cmd.envs(env_overrides.iter());
    }
    cmd.env_remove("PYTHONHOME");

    if exec {
        match exec_spawn(&mut cmd)? {};
    } else {
        Ok(cmd.status()?)
    }
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
            echo!("{}", name);
        } else {
            echo!("{} ({})", name, style(script).dim());
        }
    }
    Ok(())
}
