use std::path::Path;
use std::process::Command;
use std::{env, fs};

use anyhow::{bail, Context, Error};
use console::style;
use serde::{Deserialize, Serialize};
use tempfile::tempdir;

use crate::bootstrap::{ensure_self_venv, fetch, get_pip_module};
use crate::config::get_toolchain_python_bin;
use crate::consts::VENV_BIN;
use crate::lock::{
    update_single_project_lockfile, update_workspace_lockfile, LockMode, LockOptions,
};
use crate::piptools::get_pip_sync;
use crate::pyproject::{get_current_venv_python_version, PyProject};
use crate::sources::PythonVersion;
use crate::utils::{get_venv_python_bin, symlink_dir, CommandOutput};

/// Controls the sync mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum SyncMode {
    /// Just ensures Python is there
    #[default]
    PythonOnly,
    /// Lock only
    LockOnly,
    /// Update dependencies
    Regular,
    /// recreate everything
    Full,
}

/// Updates the virtualenv based on the pyproject.toml
#[derive(Debug, Default)]
pub struct SyncOptions {
    /// How verbose should the sync be?
    pub output: CommandOutput,
    /// Include dev dependencies?
    pub dev: bool,
    /// Which sync mode should be used?
    pub mode: SyncMode,
    /// Forces venv creation even when unsafe.
    pub force: bool,
    /// Do not lock.
    pub no_lock: bool,
    /// Controls locking.
    pub lock_options: LockOptions,
}

impl SyncOptions {
    /// Only sync the Python itself.
    pub fn python_only() -> SyncOptions {
        SyncOptions {
            mode: SyncMode::PythonOnly,
            ..Default::default()
        }
    }
}

/// Config written into the virtualenv for sync purposes.
#[derive(Serialize, Deserialize, Debug)]
pub struct VenvMarker {
    pub python: PythonVersion,
}

/// Synchronizes a project's virtualenv.
pub fn sync(cmd: SyncOptions) -> Result<(), Error> {
    let pyproject = PyProject::discover()?;
    let lockfile = pyproject.workspace_path().join("requirements.lock");
    let dev_lockfile = pyproject.workspace_path().join("requirements-dev.lock");
    let venv = pyproject.venv_path();
    let py_ver = pyproject.venv_python_version()?;
    let output = cmd.output;

    // ensure we are bootstrapped
    let self_venv = ensure_self_venv(output).context("could not sync because bootstrap failed")?;

    let mut recreate = cmd.mode == SyncMode::Full;
    if venv.is_dir() {
        if let Some(marker_python) = get_current_venv_python_version(&venv) {
            if marker_python != py_ver {
                if cmd.output != CommandOutput::Quiet {
                    eprintln!(
                        "Python version mismatch (found {}, expect {}), recreating.",
                        marker_python, py_ver
                    );
                }
                recreate = true;
            }
        } else if cmd.force {
            if cmd.output != CommandOutput::Quiet {
                eprintln!("Forcing re-creation of non rye managed virtualenv");
            }
            recreate = true;
        } else {
            bail!("virtualenv is not managed by rye. Run `rye sync -f` to force.");
        }
    }

    // make sure we have a compatible python version
    let py_ver =
        fetch(&py_ver.into(), output).context("failed fetching toolchain ahead of sync")?;

    // kill the virtualenv if it's there and we need to get rid of it.
    if recreate {
        fs::remove_dir_all(&venv).ok();
    }

    if venv.is_dir() {
        // we only care about this output if regular syncs are used
        if !matches!(cmd.mode, SyncMode::PythonOnly | SyncMode::LockOnly)
            && output != CommandOutput::Quiet
        {
            eprintln!("Reusing already existing virtualenv");
        }
    } else {
        if output != CommandOutput::Quiet {
            eprintln!(
                "Initializing new virtualenv in {}",
                style(venv.display()).cyan()
            );
            eprintln!("Python version: {}", style(&py_ver).cyan());
        }
        create_virtualenv(output, &self_venv, &py_ver, &venv)
            .context("failed creating virtualenv ahead of sync")?;
        fs::write(
            venv.join("rye-venv.json"),
            serde_json::to_string_pretty(&VenvMarker {
                python: py_ver.clone(),
            })?,
        )
        .context("failed writing venv marker file")?;
    }

    // prepare necessary utilities for pip-sync.  This is a super crude
    // hack to make this work for now.  We basically sym-link pip itself
    // into a folder all by itself and place a second file in there which we
    // can pass to pip-sync to install the local package.
    if recreate || cmd.mode != SyncMode::PythonOnly {
        if cmd.no_lock {
            let lockfile = if cmd.dev { &dev_lockfile } else { &lockfile };
            if !lockfile.is_file() {
                bail!(
                    "Locking is disabled but lockfile '{}' does not exist",
                    lockfile.display()
                );
            }
        } else if let Some(workspace) = pyproject.workspace() {
            // make sure we have an up-to-date lockfile
            update_workspace_lockfile(
                &py_ver,
                workspace,
                LockMode::Production,
                &lockfile,
                cmd.output,
                &cmd.lock_options,
            )
            .context("could not write production lockfile for workspace")?;
            update_workspace_lockfile(
                &py_ver,
                workspace,
                LockMode::Dev,
                &dev_lockfile,
                cmd.output,
                &cmd.lock_options,
            )
            .context("could not write dev lockfile for workspace")?;
        } else {
            // make sure we have an up-to-date lockfile
            update_single_project_lockfile(
                &py_ver,
                &pyproject,
                LockMode::Production,
                &lockfile,
                cmd.output,
                &cmd.lock_options,
            )
            .context("could not write production lockfile for project")?;
            update_single_project_lockfile(
                &py_ver,
                &pyproject,
                LockMode::Dev,
                &dev_lockfile,
                cmd.output,
                &cmd.lock_options,
            )
            .context("could not write dev lockfile for project")?;
        }

        // run pip install with the lockfile.
        if cmd.mode != SyncMode::LockOnly {
            if output != CommandOutput::Quiet {
                eprintln!("Installing dependencies");
            }
            let tempdir = tempdir()?;
            symlink_dir(get_pip_module(&self_venv), tempdir.path().join("pip"))
                .context("failed linking pip module into for pip-sync")?;
            let mut pip_sync_cmd = Command::new(get_pip_sync(&py_ver, output)?);
            let root = pyproject.workspace_path();

            let py_path = get_venv_python_bin(&venv);

            pip_sync_cmd
                // XXX: ${PROJECT_ROOT} is supposed to be used in the context of file:///
                // so let's make sure it is url escaped.  This is pretty hacky but
                // good enough for now.
                .env("PROJECT_ROOT", root.to_string_lossy().replace(' ', "%2F"))
                .env("PYTHONPATH", tempdir.path())
                .current_dir(&root)
                .arg("--python-executable")
                .arg(&py_path)
                .arg("--pip-args")
                // note that the double quotes are necessary to properly handle
                // spaces in paths
                .arg(format!("--python=\"{}\" --no-deps", py_path.display()));

            if cmd.dev && dev_lockfile.is_file() {
                pip_sync_cmd.arg(&dev_lockfile);
            } else {
                pip_sync_cmd.arg(&lockfile);
            }

            if output == CommandOutput::Verbose {
                pip_sync_cmd.arg("--verbose");
                if env::var("PIP_VERBOSE").is_err() {
                    pip_sync_cmd.env("PIP_VERBOSE", "2");
                }
            } else if output != CommandOutput::Quiet {
                pip_sync_cmd.env("PYTHONWARNINGS", "ignore");
            } else {
                pip_sync_cmd.arg("-q");
            }
            let status = pip_sync_cmd.status().context("unable to run pip-sync")?;
            if !status.success() {
                bail!("Installation of dependencies failed");
            }
        }
    }

    if output != CommandOutput::Quiet && cmd.mode != SyncMode::PythonOnly {
        eprintln!("Done!");
    }

    Ok(())
}

pub fn create_virtualenv(
    output: CommandOutput,
    self_venv: &Path,
    py_ver: &PythonVersion,
    venv: &Path,
) -> Result<(), Error> {
    let py_bin = get_toolchain_python_bin(py_ver)?;
    let mut venv_cmd = Command::new(self_venv.join(VENV_BIN).join("virtualenv"));
    if output == CommandOutput::Verbose {
        venv_cmd.arg("--verbose");
    } else {
        venv_cmd.arg("-q");
        venv_cmd.env("PYTHONWARNINGS", "ignore");
    }
    venv_cmd.arg("-p");
    venv_cmd.arg(&py_bin);
    venv_cmd.arg("--no-seed");
    venv_cmd.arg("--");
    venv_cmd.arg(venv);
    let status = venv_cmd
        .status()
        .context("unable to invoke virtualenv command")?;
    if !status.success() {
        bail!("failed to initialize virtualenv");
    }
    Ok(())
}
