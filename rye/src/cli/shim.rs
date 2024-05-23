use std::borrow::Cow;
use std::convert::Infallible;
use std::env;
use std::ffi::{OsStr, OsString};
use std::str::FromStr;

use anyhow::{anyhow, bail, Context, Error};
use same_file::is_same_file;
use std::process::Command;

use crate::bootstrap::{ensure_self_venv, get_pip_runner};
use crate::config::Config;
use crate::consts::VENV_BIN;
use crate::platform::{get_python_version_request_from_pyenv_pin, get_toolchain_python_bin};
use crate::pyproject::{latest_available_python_version, PyProject};
use crate::sources::py::PythonVersionRequest;
use crate::sync::{sync, SyncOptions};
use crate::tui::redirect_to_stderr;
use crate::utils::{exec_spawn, get_venv_python_bin, CommandOutput};

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
    let runner = get_pip_runner(&venv).context("could not locate pip")?;
    let python = get_venv_python_bin(&pyproject.venv_path());

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

        // on windows we also want to filter out the windows store python
        #[cfg(windows)]
        {
            if is_pointless_windows_store_applink(&bin) {
                continue;
            }
        }

        let mut args = args.to_vec();
        args[0] = bin.into();
        return Ok(Some(args));
    }

    Ok(None)
}

/// On windows we might encounter the windows store proxy shim.  This requires
/// quite a bit of custom logic to figure out what this thing does.
///
/// This is a pretty dumb way.  We know how to parse this reparse point, but Microsoft
/// does not want us to do this as the format is unstable.  So this is a best effort way.
/// we just hope that the reparse point has the python redirector in it, when it's not
/// pointing to a valid Python.
#[cfg(windows)]
fn is_pointless_windows_store_applink(path: &std::path::Path) -> bool {
    use std::os::windows::fs::MetadataExt;
    use std::os::windows::prelude::OsStrExt;
    use winapi::um::fileapi::{CreateFileW, OPEN_EXISTING};
    use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
    use winapi::um::ioapiset::DeviceIoControl;
    use winapi::um::winbase::{FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT};
    use winapi::um::winioctl::FSCTL_GET_REPARSE_POINT;
    use winapi::um::winnt::{FILE_ATTRIBUTE_REPARSE_POINT, MAXIMUM_REPARSE_DATA_BUFFER_SIZE};

    // only if we are in the special WindowsApps folder and we are called
    // python, we can be a relevant store proxy
    if !path.as_os_str().to_str().map_or(false, |x| {
        x.contains("Local\\Microsoft\\WindowsApps\\python")
    }) {
        return false;
    }

    // only if the file is a reparse point, is it relevant.
    let md = match std::fs::symlink_metadata(path) {
        Ok(md) => md,
        Err(_) => return false,
    };
    if md.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0 {
        return false;
    }

    let mut path_encoded = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let reparse_handle = unsafe {
        CreateFileW(
            path_encoded.as_mut_ptr(),
            0,
            0,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            std::ptr::null_mut(),
        )
    };

    if reparse_handle == INVALID_HANDLE_VALUE {
        return false;
    }

    let mut buf = [0u16; MAXIMUM_REPARSE_DATA_BUFFER_SIZE as usize];
    let mut bytes_returned = 0;
    let success = unsafe {
        DeviceIoControl(
            reparse_handle,
            FSCTL_GET_REPARSE_POINT,
            std::ptr::null_mut(),
            0,
            buf.as_mut_ptr() as *mut _,
            buf.len() as u32 * 2,
            &mut bytes_returned,
            std::ptr::null_mut(),
        ) != 0
    };

    unsafe {
        CloseHandle(reparse_handle);
    }

    success && String::from_utf16_lossy(&buf).contains("\\AppInstallerPythonRedirector.exe")
}

fn is_python_shim(target: &str) -> bool {
    matches_shim(target, "python") || matches_shim(target, "python3")
}

/// Figures out where a shim should point to.
fn get_shim_target(
    target: &str,
    args: &[OsString],
    pyproject: Option<&PyProject>,
) -> Result<Option<Vec<OsString>>, Error> {
    // if we can find a project, we always look for a local virtualenv first for shims.
    if let Some(pyproject) = pyproject {
        // However we only allow automatic syncing, if we are rye managed.
        if pyproject.rye_managed() {
            let _guard = redirect_to_stderr(true);
            sync(SyncOptions::python_only()).context("sync ahead of shim resolution failed")?;
        }

        if is_python_shim(target)
            && args
                .get(1)
                .and_then(|x| x.as_os_str().to_str())
                .map_or(false, |x| x.starts_with('+'))
        {
            bail!("Explicit Python selection is not possible within Rye managed projects.");
        }

        let mut args = args.to_vec();
        let folder = pyproject.venv_path().join(VENV_BIN);
        if let Some(m) = which::which_in_global(target, Some(&folder))?.next() {
            args[0] = m.into();
            return Ok(Some(args));
        }

        // on windows a virtualenv does not contain a python3 executable normally.  In that
        // case however we want to ensure that we do not shadow out to the global python3
        // executable which might be from the python store.
        #[cfg(windows)]
        {
            if matches_shim(target, "python3") {
                if let Some(m) = which::which_in_global("python", Some(folder))?.next() {
                    args[0] = m.into();
                    return Ok(Some(args));
                }
            }
        }

        // secret pip shims
        if matches_shim(target, "pip") || matches_shim(target, "pip3") {
            return Ok(Some(get_pip_shim(pyproject, args, CommandOutput::Normal)?));
        }

    // Global shims (either implicit or requested)
    } else if is_python_shim(target) {
        let config = Config::current();
        let mut remove1 = false;

        let (version_request, implicit_request) = if let Some(rest) = args
            .get(1)
            .and_then(|x| x.as_os_str().to_str())
            .and_then(|x| x.strip_prefix('+'))
        {
            remove1 = true;
            (
                PythonVersionRequest::from_str(rest)
                    .context("invalid Python version requested from command line")?,
                false,
            )
        } else if config.global_python() {
            (
                match get_python_version_request_from_pyenv_pin(&std::env::current_dir()?) {
                    Some(version_request) => version_request,
                    None => config.default_toolchain()?,
                },
                true,
            )
        } else {
            // if neither requested explicitly, nor global-python is enabled, we fall
            // back to the next shadowed target
            return find_shadowed_target(target, args);
        };

        let py_ver = latest_available_python_version(&version_request)
            .ok_or_else(|| anyhow!("Unable to determine target Python version"))?;
        let py = get_toolchain_python_bin(&py_ver)?;
        if !py.is_file() {
            let hint = if implicit_request {
                Cow::Borrowed("rye fetch")
            } else {
                Cow::Owned(format!("rye fetch {}", py_ver))
            };
            bail!(
                "Requested Python version ({}) is not installed. Install with `{}`",
                py_ver,
                hint
            );
        }

        let mut args = args.to_vec();
        args[0] = py.into();
        if remove1 {
            args.remove(1);
        }
        return Ok(Some(args));
    }

    // if we make it this far, we did not find a shim in the project, look for
    // a global one instead.
    find_shadowed_target(target, args)
}

fn spawn_shim(args: Vec<OsString>) -> Result<Infallible, Error> {
    let mut cmd = Command::new(&args[0]);
    cmd.args(&args[1..]);
    match exec_spawn(&mut cmd)? {}
}

#[cfg(not(windows))]
fn matches_shim(s: &str, reference: &str) -> bool {
    // we don't actually know if the file system is case sensitive or not, but
    // at least on mac we can assume it is, so we err on the side of that for now.
    s.eq_ignore_ascii_case(reference)
}

#[cfg(windows)]
fn matches_shim(s: &str, reference: &str) -> bool {
    if s.get(s.len().saturating_sub(4)..)
        .unwrap_or("")
        .eq_ignore_ascii_case(".exe")
    {
        &s[..s.len() - 4]
    } else {
        s
    }
    .eq_ignore_ascii_case(reference)
}

/// This replaces ourselves with the shim target for when the
/// executable is invoked as a shim executable.
pub fn execute_shim(args: &[OsString]) -> Result<(), Error> {
    if let Some(shim_name) = detect_shim(args) {
        let pyproject = PyProject::discover().ok();
        if let Some(args) = get_shim_target(&shim_name, args, pyproject.as_ref())? {
            match spawn_shim(args)? {}
        } else if is_python_shim(&shim_name) {
            if pyproject.is_some() {
                bail!("Target Python binary '{}' not found in project. Most likely running 'rye sync' will resolve this.", shim_name);
            } else {
                bail!(
                    "Target Python binary '{}' not found.\nYou are currently outside of a project. \
                    To resolve this, consider enabling global shims. \
                    Global shims allow for a Rye-managed Python installation.\n\
                    For more information: https://rye.astral.sh/guide/shims/#global-shims", shim_name
                );
            }
        } else {
            bail!("target shim binary '{}' not found", shim_name);
        }
    }
    Ok(())
}
