use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use anyhow::{bail, Context, Error};
use console::style;
use same_file::is_same_file;
use serde::{Deserialize, Serialize};
use tempfile::tempdir;

use crate::bootstrap::{ensure_self_venv, fetch, get_pip_module, FetchOptions};
use crate::config::Config;
use crate::consts::VENV_BIN;
use crate::lock::{
    make_project_root_fragment, update_single_project_lockfile, update_workspace_lockfile,
    KeyringProvider, LockMode, LockOptions,
};
use crate::piptools::{get_pip_sync, get_pip_tools_venv_path};
use crate::platform::get_toolchain_python_bin;
use crate::pyproject::{read_venv_marker, write_venv_marker, ExpandedSources, PyProject};
use crate::sources::py::PythonVersion;
use crate::utils::{
    get_venv_python_bin, set_proxy_variables, symlink_dir, update_venv_sync_marker, CommandOutput,
    IoPathContext,
};
use crate::uv::UvBuilder;

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
    /// Keyring provider to use for credential lookup.
    pub keyring_provider: KeyringProvider,
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
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct VenvMarker {
    pub python: PythonVersion,
    pub venv_path: Option<PathBuf>,
}

impl VenvMarker {
    pub fn is_compatible(&self, py_ver: &PythonVersion) -> bool {
        self.python == *py_ver
    }
}

/// Synchronizes a project's virtualenv.
pub fn sync(mut cmd: SyncOptions) -> Result<(), Error> {
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

    // Turn on generate_hashes if the project demands it.
    if pyproject.generate_hashes() {
        cmd.lock_options.generate_hashes = true;
    }

    // Turn on locking with sources if the project demands it.
    if pyproject.lock_with_sources() {
        cmd.lock_options.with_sources = true;
    }

    // ensure we are bootstrapped
    let self_venv = ensure_self_venv(output).context("could not sync because bootstrap failed")?;

    let mut recreate = cmd.mode == SyncMode::Full;
    if venv.is_dir() {
        if let Some(marker) = read_venv_marker(&venv) {
            if marker.python != py_ver {
                echo!(
                    if cmd.output,
                    "Python version mismatch (found {}, expected {}), recreating.",
                    marker.python,
                    py_ver
                );
                recreate = true;
            } else if let Some(ref venv_path) = marker.venv_path {
                // for virtualenvs that have a location identifier, check if we need to
                // recreate it.  On IO error we know that one of the paths is gone, so
                // something needs recreation.
                if !is_same_file(&venv, venv_path).unwrap_or(false) {
                    echo!(
                        if cmd.output,
                        "Detected relocated virtualenv ({} => {}), recreating.",
                        venv_path.display(),
                        venv.display(),
                    );
                    recreate = true;
                }
            }
        } else if cmd.force {
            echo!(if cmd.output, "Forcing re-creation of non-rye managed virtualenv");
            recreate = true;
        } else if cmd.mode == SyncMode::PythonOnly {
            // in python-only sync mode, don't complain about foreign venvs
            return Ok(());
        } else {
            bail!("virtualenv is not managed by rye. Run `rye sync -f` to force.");
        }
    }

    // make sure we have a compatible python version
    let py_ver = fetch(&py_ver.into(), FetchOptions::with_output(output))
        .context("failed fetching toolchain ahead of sync")?;

    // kill the virtualenv if it's there and we need to get rid of it.
    if recreate && venv.is_dir() {
        fs::remove_dir_all(&venv).path_context(&venv, "failed to delete existing virtualenv")?;
    }

    if venv.is_dir() {
        // we only care about this output if regular syncs are used
        if !matches!(cmd.mode, SyncMode::PythonOnly | SyncMode::LockOnly) {
            echo!(if output, "Reusing already existing virtualenv");
        }
    } else {
        echo!(
            if output,
            "Initializing new virtualenv in {}",
            style(venv.display()).cyan()
        );
        echo!(if output, "Python version: {}", style(&py_ver).cyan());
        let prompt = pyproject.name().unwrap_or("venv");
        create_virtualenv(output, &self_venv, &py_ver, &venv, prompt)
            .context("failed creating virtualenv ahead of sync")?;
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
                cmd.keyring_provider,
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
                cmd.keyring_provider,
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
                cmd.keyring_provider,
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
                cmd.keyring_provider,
            )
            .context("could not write dev lockfile for project")?;
        }

        // run pip install with the lockfile.
        if cmd.mode != SyncMode::LockOnly {
            echo!(if output, "Installing dependencies");

            let target_lockfile = if cmd.dev && dev_lockfile.is_file() {
                dev_lockfile
            } else {
                lockfile
            };

            let tempdir = tempdir()?;
            let py_path = get_venv_python_bin(&venv);
            if Config::current().use_uv() {
                UvBuilder::new()
                    .with_output(output.quieter())
                    .with_workdir(&pyproject.workspace_path())
                    .with_sources(sources)
                    .ensure_exists()?
                    .venv(&venv, &py_path, &py_ver, None)?
                    .with_output(output)
                    .sync(&target_lockfile)?;
            } else {
                let mut pip_sync_cmd = Command::new(get_pip_sync(&py_ver, output)?);
                let root = pyproject.workspace_path();

                // we need to run this after we have run the `get_pip_sync` command
                // as this is what bootstraps or updates the pip tools installation.
                // This is needed as on unix platforms we need to search the module path.
                symlink_dir(
                    get_pip_module(&get_pip_tools_venv_path(&py_ver))
                        .context("could not locate pip")?,
                    tempdir.path().join("pip"),
                )
                .context("failed linking pip module into for pip-sync")?;

                pip_sync_cmd
                    .env("PROJECT_ROOT", make_project_root_fragment(&root))
                    .env("PYTHONPATH", tempdir.path())
                    .current_dir(&root)
                    .arg("--python-executable")
                    .arg(&py_path)
                    .arg("--pip-args")
                    .arg("--no-deps");

                if output != CommandOutput::Quiet {
                    pip_sync_cmd.env("PYTHONWARNINGS", "ignore");
                } else if output == CommandOutput::Verbose && env::var("PIP_VERBOSE").is_err() {
                    pip_sync_cmd.env("PIP_VERBOSE", "2");
                }

                sources.add_as_pip_args(&mut pip_sync_cmd);

                pip_sync_cmd.arg(&target_lockfile);

                if output == CommandOutput::Verbose {
                    pip_sync_cmd.arg("--verbose");
                } else if output == CommandOutput::Quiet {
                    pip_sync_cmd.arg("-q");
                }
                set_proxy_variables(&mut pip_sync_cmd);
                let status = pip_sync_cmd.status().context("unable to run pip-sync")?;

                if !status.success() {
                    bail!("Installation of dependencies failed");
                }
            };
        };
    }

    if cmd.mode != SyncMode::PythonOnly {
        echo!(if output, "Done!");
    }

    Ok(())
}

/// Performs an autosync.
pub fn autosync(
    pyproject: &PyProject,
    output: CommandOutput,
    pre: bool,
    with_sources: bool,
    generate_hashes: bool,
    keyring_provider: KeyringProvider,
) -> Result<(), Error> {
    sync(SyncOptions {
        output,
        dev: true,
        mode: SyncMode::Regular,
        force: false,
        no_lock: false,
        lock_options: LockOptions {
            pre,
            with_sources,
            generate_hashes,
            ..Default::default()
        },
        pyproject: Some(pyproject.toml_path().to_path_buf()),
        keyring_provider,
    })
}

pub fn create_virtualenv(
    output: CommandOutput,
    self_venv: &Path,
    py_ver: &PythonVersion,
    venv: &Path,
    prompt: &str,
) -> Result<(), Error> {
    let py_bin = get_toolchain_python_bin(py_ver)?;

    if Config::current().use_uv() {
        // try to kill the empty venv if there is one as uv can't work otherwise.
        fs::remove_dir(venv).ok();
        let uv = UvBuilder::new()
            .with_output(output.quieter())
            .ensure_exists()?
            .venv(venv, &py_bin, py_ver, Some(prompt))
            .context("failed to initialize virtualenv")?;
        uv.write_marker()?;
        uv.sync_marker();
    } else {
        // create the venv folder first so we can manipulate some flags on it.
        fs::create_dir_all(venv).path_context(venv, "unable to create virtualenv folder")?;

        update_venv_sync_marker(output, venv);
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
        venv_cmd.arg("--prompt");
        venv_cmd.arg(prompt);
        venv_cmd.arg("--").arg(venv);
        let status = venv_cmd
            .status()
            .context("unable to invoke virtualenv command")?;
        if !status.success() {
            bail!("failed to initialize virtualenv");
        }

        write_venv_marker(venv, py_ver)?;
    };

    // On UNIX systems Python is unable to find the tcl config that is placed
    // outside of the virtualenv.  It also sometimes is entirely unable to find
    // the tcl config that comes from the standalone python builds.
    #[cfg(unix)]
    {
        inject_tcl_config(venv, &py_bin)?;
    }

    Ok(())
}

#[cfg(unix)]
fn inject_tcl_config(venv: &Path, py_bin: &Path) -> Result<(), Error> {
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

    if tk_lib.is_none() && tcl_lib.is_none() {
        return Ok(());
    }

    if let Some(site_packages) = get_site_packages(venv.join("lib"))? {
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
    }

    Ok(())
}

// There is only one folder in the venv/lib folder. But in practice, only pypy will use this method in linux
#[cfg(unix)]
fn get_site_packages(lib_dir: PathBuf) -> Result<Option<PathBuf>, Error> {
    let entries = fs::read_dir(&lib_dir).path_context(&lib_dir, "read venv/lib/ path failed")?;

    for entry in entries {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            return Ok(Some(entry.path().join("site-packages")));
        }
    }
    Ok(None)
}
