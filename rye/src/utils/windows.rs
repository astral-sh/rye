use std::env;
use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Error};
use winreg::enums::{RegType, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
use winreg::{RegKey, RegValue};

const RYE_UNINSTALL_ENTRY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Rye";

pub(crate) fn add_to_path(rye_home: &Path) -> Result<(), Error> {
    let target_path = reverse_resolve_user_profile(rye_home.join("shims"));
    if let Some(old_path) = get_windows_path_var()? {
        if let Some(new_path) =
            append_entry_to_path(old_path, target_path.as_os_str().encode_wide().collect())
        {
            apply_new_path(new_path)?;
        }
    }
    Ok(())
}

pub(crate) fn remove_from_path(rye_home: &Path) -> Result<(), Error> {
    let target_path = reverse_resolve_user_profile(rye_home.join("shims"));
    if let Some(old_path) = get_windows_path_var()? {
        if let Some(new_path) =
            remove_entry_from_path(old_path, target_path.as_os_str().encode_wide().collect())
        {
            apply_new_path(new_path)?;
        }
    }
    Ok(())
}

/// If the target path is under the user profile, replace it with %USERPROFILE%.  The
/// motivation here is that this was the path we documented originally so someone updating
/// Rye does not end up with two competing paths in the list for no reason.
fn reverse_resolve_user_profile(path: PathBuf) -> PathBuf {
    if let Some(user_profile) = env::var_os("USERPROFILE").map(PathBuf::from) {
        if let Ok(rest) = path.strip_prefix(&user_profile) {
            return Path::new("%USERPROFILE%").join(rest);
        }
    }
    path
}

fn apply_new_path(new_path: Vec<u16>) -> Result<(), Error> {
    use std::ptr;
    use winapi::shared::minwindef::*;
    use winapi::um::winuser::{
        SendMessageTimeoutA, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
    };

    let root = RegKey::predef(HKEY_CURRENT_USER);
    let environment = root.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;

    if new_path.is_empty() {
        environment.delete_value("PATH")?;
    } else {
        let reg_value = RegValue {
            bytes: to_winreg_bytes(new_path),
            vtype: RegType::REG_EXPAND_SZ,
        };
        environment.set_raw_value("PATH", &reg_value)?;
    }

    // Tell other processes to update their environment
    #[allow(clippy::unnecessary_cast)]
    unsafe {
        SendMessageTimeoutA(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0 as WPARAM,
            "Environment\0".as_ptr() as LPARAM,
            SMTO_ABORTIFHUNG,
            5000,
            ptr::null_mut(),
        );
    }

    Ok(())
}

/// Get the windows PATH variable out of the registry as a String. If
/// this returns None then the PATH variable is not a string and we
/// should not mess with it.
fn get_windows_path_var() -> Result<Option<Vec<u16>>, Error> {
    use std::io;

    let root = RegKey::predef(HKEY_CURRENT_USER);
    let environment = root
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .context("Failed opening Environment key")?;

    let reg_value = environment.get_raw_value("PATH");
    match reg_value {
        Ok(val) => {
            if let Some(s) = from_winreg_value(&val) {
                Ok(Some(s))
            } else {
                warn!(
                    "the registry key HKEY_CURRENT_USER\\Environment\\PATH is not a string. \
                       Not modifying the PATH variable"
                );
                Ok(None)
            }
        }
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(Some(Vec::new())),
        Err(e) => Err(e).context("failure during windows path manipulation"),
    }
}

/// Returns None if the existing old_path does not need changing, otherwise
/// prepends the path_str to old_path, handling empty old_path appropriately.
fn append_entry_to_path(old_path: Vec<u16>, path_str: Vec<u16>) -> Option<Vec<u16>> {
    if old_path.is_empty() {
        Some(path_str)
    } else if old_path
        .windows(path_str.len())
        .any(|path| path == path_str)
    {
        None
    } else {
        let mut new_path = path_str;
        new_path.push(b';' as u16);
        new_path.extend_from_slice(&old_path);
        Some(new_path)
    }
}

/// Returns None if the existing old_path does not need changing
fn remove_entry_from_path(old_path: Vec<u16>, path_str: Vec<u16>) -> Option<Vec<u16>> {
    let idx = old_path
        .windows(path_str.len())
        .position(|path| path == path_str)?;
    // If there's a trailing semicolon (likely, since we probably added one
    // during install), include that in the substring to remove. We don't search
    // for that to find the string, because if it's the last string in the path,
    // there may not be.
    let mut len = path_str.len();
    if old_path.get(idx + path_str.len()) == Some(&(b';' as u16)) {
        len += 1;
    }

    let mut new_path = old_path[..idx].to_owned();
    new_path.extend_from_slice(&old_path[idx + len..]);
    // Don't leave a trailing ; though, we don't want an empty string in the
    // path.
    if new_path.last() == Some(&(b';' as u16)) {
        new_path.pop();
    }
    Some(new_path)
}

/// Registers rye as installed program.
pub(crate) fn add_to_programs(rye_home: &Path) -> Result<(), Error> {
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(RYE_UNINSTALL_ENTRY)
        .context("Failed creating uninstall key")?
        .0;

    // Don't overwrite registry if Rye is already installed
    let prev = key
        .get_raw_value("UninstallString")
        .map(|val| from_winreg_value(&val));
    if let Ok(Some(s)) = prev {
        let mut path = PathBuf::from(OsString::from_wide(&s));
        path.pop();
        if path.exists() {
            return Ok(());
        }
    }

    let mut uninstall_cmd = OsString::from("\"");
    uninstall_cmd.push(rye_home);
    uninstall_cmd.push("\" self uninstall");

    let reg_value = RegValue {
        bytes: to_winreg_bytes(uninstall_cmd.encode_wide().collect()),
        vtype: RegType::REG_SZ,
    };

    let current_version: &str = env!("CARGO_PKG_VERSION");

    key.set_raw_value("UninstallString", &reg_value)
        .context("Failed to set uninstall string")?;
    key.set_value(
        "DisplayName",
        &"Rye: An Experimental Package Management Solution for Python",
    )
    .context("Failed to set display name")?;
    key.set_value("DisplayVersion", &current_version)
        .context("Failed to set display version")?;
    key.set_value("Publisher", &"Rye")
        .context("Failed to set publisher")?;

    Ok(())
}

/// Removes the entry on uninstall from the program list.
pub(crate) fn remove_from_programs() -> Result<(), Error> {
    match RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(RYE_UNINSTALL_ENTRY) {
        Ok(()) => Ok(()),
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(anyhow!(e)),
    }
}

/// Convert a vector UCS-2 chars to a null-terminated UCS-2 string in bytes
pub(crate) fn to_winreg_bytes(mut v: Vec<u16>) -> Vec<u8> {
    v.push(0);
    unsafe { std::slice::from_raw_parts(v.as_ptr().cast::<u8>(), v.len() * 2).to_vec() }
}

/// This is used to decode the value of HKCU\Environment\PATH. If that key is
/// not REG_SZ | REG_EXPAND_SZ then this returns None. The winreg library itself
/// does a lossy unicode conversion.
pub(crate) fn from_winreg_value(val: &winreg::RegValue) -> Option<Vec<u16>> {
    use std::slice;

    match val.vtype {
        RegType::REG_SZ | RegType::REG_EXPAND_SZ => {
            // Copied from winreg
            let mut words = unsafe {
                #[allow(clippy::cast_ptr_alignment)]
                slice::from_raw_parts(val.bytes.as_ptr().cast::<u16>(), val.bytes.len() / 2)
                    .to_owned()
            };
            while words.last() == Some(&0) {
                words.pop();
            }
            Some(words)
        }
        _ => None,
    }
}
