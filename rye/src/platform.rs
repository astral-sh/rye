use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::{env, fs};

use anyhow::{anyhow, Context, Error};

use crate::pyproject::latest_available_python_version;
use crate::sources::{PythonVersion, PythonVersionRequest};

static APP_DIR: Mutex<Option<&'static PathBuf>> = Mutex::new(None);

pub fn init() -> Result<(), Error> {
    let home = if let Some(rye_home) = env::var_os("RYE_HOME") {
        PathBuf::from(rye_home)
    } else {
        simple_home_dir::home_dir()
            .map(|x| x.join(".rye"))
            .ok_or_else(|| anyhow!("could not determine home folder"))?
    };
    *APP_DIR.lock().unwrap() = Some(Box::leak(Box::new(home)));
    Ok(())
}

/// Returns the application directory.
pub fn get_app_dir() -> &'static Path {
    APP_DIR.lock().unwrap().expect("platform not initialized")
}

/// Runs a check if symlinks are supported.
pub fn symlinks_supported() -> bool {
    #[cfg(unix)]
    {
        true
    }
    #[cfg(windows)]
    {
        use once_cell::sync::Lazy;

        fn probe() -> Result<(), std::io::Error> {
            let dir = tempfile::tempdir()?;
            let a_path = dir.path().join("a");
            fs::write(&a_path, "")?;
            std::os::windows::fs::symlink_file(&a_path, dir.path().join("b"))?;
            Ok(())
        }

        static SUPPORTED: Lazy<bool> = Lazy::new(|| probe().is_ok());
        *SUPPORTED
    }
}

/// Returns the cache directory for a particular python version that can be downloaded.
pub fn get_canonical_py_path(version: &PythonVersion) -> Result<PathBuf, Error> {
    let mut rv = get_app_dir().to_path_buf();
    rv.push("py");
    rv.push(version.to_string());
    Ok(rv)
}

/// Returns the path of the python binary for the given version.
pub fn get_toolchain_python_bin(version: &PythonVersion) -> Result<PathBuf, Error> {
    let mut p = get_canonical_py_path(version)?;

    // It's permissible to link Python binaries directly in two ways.  It can either be
    // a symlink in which case it's used directly, it can be a non-executable text file
    // in which case the contents are the location of the interpreter, or it can be an
    // executable file on unix.
    if p.is_file() {
        if p.is_symlink() {
            return Ok(p.canonicalize()?);
        }
        #[cfg(unix)]
        {
            use std::os::unix::prelude::MetadataExt;
            if p.metadata().map_or(false, |x| x.mode() & 0o001 != 0) {
                return Ok(p);
            }
        }
        let contents = fs::read_to_string(&p).context("could not read toolchain file")?;
        return Ok(PathBuf::from(contents.trim_end()));
    }

    // we support install/bin/python, install/python and bin/python
    p.push("install");
    if !p.is_dir() {
        p.pop();
    }
    p.push("bin");
    if !p.is_dir() {
        p.pop();
    }

    #[cfg(unix)]
    {
        p.push("python3");
    }
    #[cfg(windows)]
    {
        p.push("python.exe");
    }

    Ok(p)
}

/// Returns a pinnable version for this version request.
///
/// This is the version number that will be written into `.python-version`
pub fn get_pinnable_version(req: &PythonVersionRequest, relaxed: bool) -> Option<String> {
    let serialized = if relaxed {
        req.to_string()
    } else {
        let mut target_version = None;

        // If the version request points directly to a known version for which we
        // have a known binary, we can use that.
        if let Ok(ver) = PythonVersion::try_from(req.clone()) {
            if let Ok(path) = get_toolchain_python_bin(&ver) {
                if path.is_file() {
                    target_version = Some(ver);
                }
            }
        }

        // otherwise, any version we can download is an acceptable version
        if target_version.is_none() {
            if let Some(version) = latest_available_python_version(req) {
                target_version = Some(version);
            }
        }

        // we return the stringified version of the version, but if always remove the
        // cpython@ prefix to make it reusable with other toolchains such as pyenv.
        if let Some(version) = target_version {
            version.to_string()
        } else {
            return None;
        }
    };

    Some(if let Some(rest) = serialized.strip_prefix("cpython@") {
        rest.to_string()
    } else {
        serialized
    })
}

/// Returns a list of all registered toolchains.
pub fn list_known_toolchains() -> Result<Vec<(PythonVersion, PathBuf)>, Error> {
    let folder = get_app_dir().join("py");
    let mut rv = Vec::new();
    if let Ok(iter) = folder.read_dir() {
        for entry in iter {
            let entry = entry?;
            if let Ok(ver) = entry
                .file_name()
                .as_os_str()
                .to_string_lossy()
                .parse::<PythonVersion>()
            {
                let target = get_toolchain_python_bin(&ver)?;
                rv.push((ver, target));
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
pub fn get_python_version_request_from_pyenv_pin(root: &Path) -> Option<PythonVersionRequest> {
    let mut here = root.to_owned();

    loop {
        here.push(".python-version");
        if let Ok(contents) = fs::read_to_string(&here) {
            let ver = contents.trim().parse().ok()?;
            return Some(ver);
        }

        // pop filename
        here.pop();

        // pop parent
        if !here.pop() {
            break;
        }
    }

    None
}

/// Returns the most recent cpython release.
pub fn get_latest_cpython_version() -> Result<PythonVersion, Error> {
    latest_available_python_version(&PythonVersionRequest {
        kind: None,
        major: 3,
        minor: None,
        patch: None,
        suffix: None,
    })
    .context("unsupported platform")
}

/// Returns the credentials data from ~/.rye.
///
/// The credentials file contains toml tables for various credential data.
/// ```toml
/// [pypi]
/// token = ""
/// ```
pub fn get_credentials() -> Result<toml_edit::Document, Error> {
    let filepath = get_credentials_filepath()?;

    // If a credentials file doesn't exist create an empty one. TODO: Move to bootstrapping?
    if !filepath.exists() {
        fs::write(&filepath, "")?;
    }

    let doc = fs::read_to_string(&filepath)?
        .parse::<toml_edit::Document>()
        .with_context(|| format!("failed to parse credentials from {}", filepath.display()))?;

    Ok(doc)
}

pub fn write_credentials(doc: &toml_edit::Document) -> Result<(), Error> {
    std::fs::write(get_credentials_filepath()?, doc.to_string())
        .context("unable to write to the credentials file")
}

pub fn get_credentials_filepath() -> Result<PathBuf, Error> {
    Ok(get_app_dir().join("credentials"))
}
