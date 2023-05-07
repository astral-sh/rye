use std::env::consts::{ARCH, OS};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

use anyhow::{anyhow, Context, Error};
use once_cell::sync::Lazy;

use crate::sources::{get_download_url, PythonVersion, PythonVersionRequest};

static APP_DIR: Lazy<Option<PathBuf>> = Lazy::new(|| {
    if let Some(rye_home) = env::var_os("RYE_HOME") {
        Some(PathBuf::from(rye_home))
    } else {
        simple_home_dir::home_dir().map(|x| x.join(".rye"))
    }
});

/// Returns the application directory.
pub fn get_app_dir() -> Result<&'static Path, Error> {
    APP_DIR
        .as_deref()
        .ok_or_else(|| anyhow!("cannot determine app directory"))
}

/// Returns the cache directory for a particular python version that can be downloaded.
pub fn get_canonical_py_path(version: &PythonVersion) -> Result<PathBuf, Error> {
    let mut rv = get_app_dir()?.to_path_buf();
    rv.push("py");
    rv.push(version.to_string());
    Ok(rv)
}

/// Returns the path of the python binary for the given version.
pub fn get_py_bin(version: &PythonVersion) -> Result<PathBuf, Error> {
    let mut p = get_canonical_py_path(version)?;

    // It's permissible to link Python binaries directly
    if p.is_file() {
        return Ok(p);
    }

    p.push("install");
    p.push("bin");
    p.push("python3");
    Ok(p)
}

/// Returns a pinnable version for this version request.
///
/// This is the version number that will be written into `.python-version`
pub fn get_pinnable_version(req: &PythonVersionRequest) -> Option<String> {
    let mut target_version = None;

    // If the version request points directly to a known version for which we
    // have a known binary, we can use that.
    if let Ok(ver) = PythonVersion::try_from(req.clone()) {
        if let Ok(path) = get_py_bin(&ver) {
            if path.is_file() {
                target_version = Some(ver);
            }
        }
    }

    // otherwise, any version we can download is an acceptable version
    if let Some((version, _)) = get_download_url(req, OS, ARCH) {
        target_version = Some(version);
    }

    // we return the stringified version of the version, but if always remove the
    // cpython@ prefix to make it reusable with other toolchains such as pyenv.
    if let Some(version) = target_version {
        let serialized_version = version.to_string();
        Some(
            if let Some(rest) = serialized_version.strip_prefix("cpython@") {
                rest.to_string()
            } else {
                serialized_version
            },
        )
    } else {
        None
    }
}

/// Returns a list of all registered toolchains.
pub fn list_known_toolchains() -> Result<Vec<PythonVersion>, Error> {
    let folder = get_app_dir()?.join("py");
    let mut rv = Vec::new();
    if let Ok(iter) = folder.read_dir() {
        for entry in iter {
            let entry = entry?;
            if let Ok(ver) = entry.file_name().as_os_str().to_string_lossy().parse() {
                rv.push(ver);
            }
        }
    }
    Ok(rv)
}

/// Returns the default author from git.
pub fn get_default_author() -> Option<(String, String)> {
    let rv = Command::new("git")
        .arg("config")
        .arg("--get-regexp")
        .arg("^user.(name|email)$")
        .stdout(Stdio::piped())
        .output()
        .ok()?;

    let mut name = None;
    let mut email = None;

    for line in std::str::from_utf8(&rv.stdout).ok()?.lines() {
        match line.split_once(' ') {
            Some((key, value)) if key == "user.email" => {
                email = Some(value.to_string());
            }
            Some((key, value)) if key == "user.name" => {
                name = Some(value.to_string());
            }
            _ => {}
        }
    }

    Some((name?, email.unwrap_or_else(|| "".into())))
}

/// Reads the current `.python-version` file.
pub fn get_python_version_from_pyenv_pin() -> Option<PythonVersion> {
    let mut here = env::current_dir().ok()?;

    loop {
        let ver_file = here.join(".python-version");
        if let Ok(contents) = fs::read_to_string(&ver_file) {
            let ver = contents.trim().parse().ok()?;
            return Some(ver);
        }

        if !here.pop() {
            break;
        }
    }

    None
}

/// Returns the most recent cpython release.
pub fn get_latest_cpython() -> Result<PythonVersion, Error> {
    get_download_url(
        &PythonVersionRequest {
            kind: None,
            major: 3,
            minor: None,
            patch: None,
            suffix: None,
        },
        OS,
        ARCH,
    )
    .map(|x| x.0)
    .context("unsupported platform")
}
