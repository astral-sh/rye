use std::env;
use std::ffi::{CString, OsStr, OsString};
use std::os::unix::prelude::OsStrExt;

use anyhow::{bail, Context, Error};
use same_file::is_same_file;

use crate::bootstrap::{ensure_self_venv, get_pip_runner};
use crate::pyproject::PyProject;
use crate::sync::{sync, SyncOptions};
use crate::utils::CommandOutput;

fn detect_shim() -> Option<(String, Vec<OsString>)> {
    // Shims are detected if the executable is linked into
    // a folder called .shims and in that case the shimmed
    // binaries is the base name.
    let args = env::args_os().collect::<Vec<_>>();
    if args.is_empty() {
        return None;
    }

    let path = env::current_exe().ok()?;
    let shim_name = path.file_name()?;
    if path.parent()?.file_name() != Some(OsStr::new("shims")) {
        return None;
    }

    Some((shim_name.to_str()?.to_owned(), args))
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
fn find_shadowed_target(
    target: &str,
    mut args: Vec<OsString>,
) -> Result<Option<Vec<OsString>>, Error> {
    let exe = env::current_exe()?;
    for bin in which::which_all(target)? {
        if is_same_file(&bin, &exe).unwrap_or(false) {
            continue;
        }
        args[0] = bin.into();
        return Ok(Some(args));
    }
    Ok(None)
}

/// Figures out where a shim should point to.
fn get_shim_target(target: &str, mut args: Vec<OsString>) -> Result<Option<Vec<OsString>>, Error> {
    let pyproject = match PyProject::discover() {
        Ok(project) => project,
        Err(_) => return find_shadowed_target(target, args),
    };

    // make sure we have the minimal virtualenv.
    sync(SyncOptions::python_only()).context("sync ahead of shim resolution failed")?;

    let path = pyproject.venv_path().join("bin").join(target);

    if path.is_file() {
        args[0] = path.into();
        return Ok(Some(args));
    }

    // secret pip shims
    if target == "pip" || target == "pip3" {
        return Ok(Some(get_pip_shim(&pyproject, args, CommandOutput::Normal)?));
    }

    Ok(None)
}

/// This replaces ourselves with the shim target for when the
/// executable is invoked as a shim executable.
pub fn execute_shim() -> Result<(), Error> {
    if let Some((shim_name, args)) = detect_shim() {
        if let Some(args) = get_shim_target(&shim_name, args)? {
            let target = &args[0];
            let args = args
                .iter()
                .filter_map(|x| CString::new(x.as_bytes()).ok())
                .collect::<Vec<_>>();
            let path = CString::new(args[0].as_bytes())?;
            nix::unistd::execv(&path, &args)
                .with_context(|| format!("unable to spawn shim {}", target.to_string_lossy()))?;
        } else {
            bail!("target shim binary not found");
        }
    }
    Ok(())
}
