use std::convert::Infallible;
use std::env;
use std::ffi::{OsStr, OsString};

use anyhow::{bail, Context, Error};
use same_file::is_same_file;
use std::process::Command;
use which::which_in_global;

use crate::bootstrap::{ensure_self_venv, get_pip_runner};
use crate::consts::VENV_BIN;
use crate::pyproject::PyProject;
use crate::sync::{sync, SyncOptions};
use crate::utils::{exec_spawn, CommandOutput};

fn detect_shim(args: &[OsString]) -> Option<String> {
    // Shims are detected if the executable is linked into
    // a folder called shims and in that case the shimmed
    // binaries is the base name.
    if args.is_empty() {
        return None;
    }

    let path = env::current_exe().ok()?;
    let shim_name = path.file_name()?;

    // rye is itself placed in the shims folder, so it must not
    // detect itself.
    if shim_name == "rye" || shim_name == "rye.exe" {
        return None;
    }

    if path.parent()?.file_name() != Some(OsStr::new("shims")) {
        return None;
    }

    Some(shim_name.to_str()?.to_owned())
}

/// Returns the pip shim.
///
/// This is special because we never install pip into our virtualenv
/// but we want to provide a pip experience in the virtualenv.  This
/// is accomplished by reconfiguring pip on the fly to point there.
fn get_pip_shim(
    pyproject: &PyProject,
    mut args: Vec<OsString>,
    output: CommandOutput,
) -> Result<Vec<OsString>, Error> {
    let venv = ensure_self_venv(output)?;
    let runner = get_pip_runner(&venv);
    let python = pyproject.venv_path().join("bin/python");

    // pip likes to emit deprecation warnings
    env::set_var("PYTHONWARNINGS", "ignore");

    // since pip is managed as part of rye itself, we do not want pip to trigger
    // version checks.  It's neither upgradable nor helpful.
    env::set_var("PIP_DISABLE_PIP_VERSION_CHECK", "1");

    args[0] = python.into();
    args.insert(1, runner.into());

    Ok(args)
}

/// Finds a target the shim which shadow.
///
/// This tries to find where a shim should point to when the shim is not
/// placed in the virtualenv.
fn find_shadowed_target(target: &str, args: &[OsString]) -> Result<Option<Vec<OsString>>, Error> {
    let exe = env::current_exe()?;
    for bin in which::which_all(target)? {
        if is_same_file(&bin, &exe).unwrap_or(false) {
            continue;
        }
        let mut args = args.to_vec();
        args[0] = bin.into();
        return Ok(Some(args));
    }
    Ok(None)
}

/// Figures out where a shim should point to.
fn get_shim_target(target: &str, args: &[OsString]) -> Result<Option<Vec<OsString>>, Error> {
    let pyproject = match PyProject::discover() {
        Ok(project) => project,
        Err(_) => return find_shadowed_target(target, args),
    };

    // make sure we have the minimal virtualenv.
    sync(SyncOptions::python_only()).context("sync ahead of shim resolution failed")?;

    let mut args = args.to_vec();
    let folder = pyproject.venv_path().join(VENV_BIN);
    if let Some(m) = which_in_global(target, Some(folder))?.next() {
        args[0] = m.into();
        return Ok(Some(args));
    }

    // secret pip shims
    if target == "pip" || target == "pip3" {
        return Ok(Some(get_pip_shim(&pyproject, args, CommandOutput::Normal)?));
    }

    Ok(None)
}

fn spawn_shim(args: Vec<OsString>) -> Result<Infallible, Error> {
    let mut cmd = Command::new(&args[0]);
    cmd.args(&args[1..]);
    match exec_spawn(&mut cmd)? {}
}

/// This replaces ourselves with the shim target for when the
/// executable is invoked as a shim executable.
pub fn execute_shim(args: &[OsString]) -> Result<(), Error> {
    if let Some(shim_name) = detect_shim(args) {
        if let Some(args) = get_shim_target(&shim_name, args)? {
            match spawn_shim(args)? {}
        } else {
            bail!("target shim binary not found");
        }
    }
    Ok(())
}
