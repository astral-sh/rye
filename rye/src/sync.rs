use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use anyhow::{bail, Context, Error};
use console::style;
use serde::{Deserialize, Serialize};
use tempfile::tempdir;

use crate::bootstrap::{ensure_self_venv, fetch, get_pip_module};
use crate::consts::VENV_BIN;
use crate::lock::{
    make_project_root_fragment, update_single_project_lockfile, update_workspace_lockfile,
    LockMode, LockOptions,
};
use crate::piptools::get_pip_sync;
use crate::platform::get_toolchain_python_bin;
use crate::pyproject::{get_current_venv_python_version, ExpandedSources, PyProject};
use crate::sources::PythonVersion;
use crate::utils::{get_venv_python_bin, set_proxy_variables, symlink_dir, CommandOutput};

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
    /// Explicit pyproject location (Only usable by PythonOnly mode)
    pub pyproject: Option<PathBuf>,
}

impl SyncOptions {
    /// Only sync the Python itself.
    pub fn python_only() -> SyncOptions {
        SyncOptions {
            mode: SyncMode::PythonOnly,
            ..Default::default()
        }
    }

    pub fn pyproject(mut self, pyproject: Option<PathBuf>) -> Self {
        self.pyproject = pyproject;
        self
    }
}

/// Config written into the virtualenv for sync purposes.
#[derive(Serialize, Deserialize, Debug)]
pub struct VenvMarker {
    pub python: PythonVersion,
}

/// Synchronizes a project's virtualenv.
pub fn sync(cmd: SyncOptions) -> Result<(), Error> {
    let pyproject = PyProject::load_or_discover(cmd.pyproject.as_deref())?;
    let lockfile = pyproject.workspace_path().join("requirements.lock");
    let dev_lockfile = pyproject.workspace_path().join("requirements-dev.lock");
    let venv = pyproject.venv_path();
    let py_ver = pyproject.venv_python_version()?;
    let output = cmd.output;

    if cmd.pyproject.is_some()
        && cmd.mode != SyncMode::PythonOnly
        && !pyproject.toml_path().ends_with("pyproject.toml")
    {
        // pip-tools will search for pyproject.toml
        bail!("cannot sync or generate lockfile: package needs 'pyproject.toml'");
    }

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
        } else if cmd.mode == SyncMode::PythonOnly {
            // in python-only sync mode, don't complain about foreign venvs
            return Ok(());
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
        let sources = ExpandedSources::from_sources(&pyproject.sources()?)?;
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
                &sources,
                &cmd.lock_options,
            )
            .context("could not write production lockfile for workspace")?;
            update_workspace_lockfile(
                &py_ver,
                workspace,
                LockMode::Dev,
                &dev_lockfile,
                cmd.output,
                &sources,
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
                &sources,
                &cmd.lock_options,
            )
            .context("could not write production lockfile for project")?;
            update_single_project_lockfile(
                &py_ver,
                &pyproject,
                LockMode::Dev,
                &dev_lockfile,
                cmd.output,
                &sources,
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
            symlink_dir(
                get_pip_module(&self_venv).context("could not locate pip")?,
                tempdir.path().join("pip"),
            )
            .context("failed linking pip module into for pip-sync")?;
            let mut pip_sync_cmd = Command::new(get_pip_sync(&py_ver, output)?);
            let root = pyproject.workspace_path();

            let py_path = get_venv_python_bin(&venv);

            pip_sync_cmd
                .env("PROJECT_ROOT", make_project_root_fragment(&root))
                .env("PYTHONPATH", tempdir.path())
                .current_dir(&root)
                .arg("--python-executable")
                .arg(&py_path)
                .arg("--pip-args")
                // note that the double quotes are necessary to properly handle
                // spaces in paths
                .arg(format!("--python=\"{}\" --no-deps", py_path.display()));

            sources.add_as_pip_args(&mut pip_sync_cmd);

            for (idx, url) in sources.index_urls.iter().enumerate() {
                if idx == 0 {
                    pip_sync_cmd.arg("--index-url");
                } else {
                    pip_sync_cmd.arg("--extra-index-url");
                }
                pip_sync_cmd.arg(&url.to_string());
            }

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
            set_proxy_variables(&mut pip_sync_cmd);
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

    // On UNIX systems Python is unable to find the tcl config that is placed
    // outside of the virtualenv.  It also sometimes is entirely unable to find
    // the tcl config that comes from the standalone python builds.
    #[cfg(unix)]
    {
        inject_tcl_config(venv, &py_bin, py_ver)?;
    }

    Ok(())
}

#[cfg(unix)]
fn inject_tcl_config(venv: &Path, py_bin: &Path, py_ver: &PythonVersion) -> Result<(), Error> {
    let lib_path = match py_bin
        .parent()
        .and_then(|x| x.parent())
        .map(|x| x.join("lib"))
    {
        Some(path) => path,
        None => return Ok(()),
    };

    let mut tcl_lib = None;
    let mut tk_lib = None;

    if let Ok(dir) = lib_path.read_dir() {
        for entry in dir.filter_map(|x| x.ok()) {
            let filename = entry.file_name();
            let name = match filename.to_str() {
                Some(name) => name,
                None => continue,
            };
            if name.starts_with("tcl8") {
                tcl_lib = Some(name.to_string());
                if tk_lib.is_some() {
                    break;
                }
            } else if name.starts_with("tk8") {
                tk_lib = Some(name.to_string());
                if tcl_lib.is_some() {
                    break;
                }
            }
        }
    }

    let site_packages = venv
        .join("lib")
        .join(format!("python{}.{}", py_ver.major, py_ver.minor))
        .join("site-packages");

    if tk_lib.is_none() && tcl_lib.is_none() {
        return Ok(());
    }

    fs::write(
        site_packages.join("_tcl-init.pth"),
        minijinja::render!(
            r#"import os, sys;
{%- if tcl_lib -%}
os.environ.setdefault('TCL_LIBRARY', sys.base_prefix + '/lib/{{ tcl_lib }}');
{%- endif -%}
{%- if tk_lib -%}
os.environ.setdefault('TK_LIBRARY', sys.base_prefix + '/lib/{{ tk_lib }}');
{%- endif -%}"#,
            tcl_lib,
            tk_lib,
        ),
    )?;

    Ok(())
}
