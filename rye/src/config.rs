use std::env::consts::{ARCH, OS};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

use anyhow::{anyhow, Error};
use once_cell::sync::Lazy;

use crate::sources::{get_download_url, PythonVersion, PythonVersionRequest};

static APP_DIR: Lazy<Option<PathBuf>> =
    Lazy::new(|| simple_home_dir::home_dir().map(|x| x.join(".rye")));

/// Returns the application directory.
pub fn get_app_dir() -> Result<&'static Path, Error> {
    APP_DIR
        .as_deref()
        .ok_or_else(|| anyhow!("cannot determine app directory"))
}

/// Returns the cache directory for a particular python version.
pub fn get_downloadable_py_dir(version: &PythonVersion) -> Result<PathBuf, Error> {
    let mut rv = get_app_dir()?.to_path_buf();
    rv.push("py");
    rv.push(version.to_string());
    Ok(rv)
}

/// Returns the path of the python binary for the given version.
pub fn get_py_bin(version: &PythonVersion) -> Result<PathBuf, Error> {
    // TODO: this only supports the redistributable pythons for now
    let mut p = get_downloadable_py_dir(version)?;
    p.push("install");
    p.push("bin");
    p.push("python3");
    Ok(p)
}

/// Returns a pinnable version for this version request.
///
/// This is the version number that will be written into `.python-version`
pub fn get_pinnable_version(req: &PythonVersionRequest) -> Option<String> {
    if let Some((version, _)) = get_download_url(req, OS, ARCH) {
        let serialized_version = version.to_string();
        if let Some(rest) = serialized_version.strip_prefix("cpython@") {
            return Some(rest.to_string());
        }
    }
    None
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
pub fn load_python_version() -> Option<PythonVersion> {
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
