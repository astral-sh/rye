use std::borrow::Cow;
use std::env::consts::{ARCH, OS};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use anyhow::{bail, Context, Error};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};

use crate::config::{get_app_dir, get_canonical_py_path, get_py_bin};
use crate::sources::{get_download_url, PythonVersion, PythonVersionRequest};
use crate::utils::{unpack_tarball, CommandOutput};

pub const SELF_PYTHON_VERSION: PythonVersionRequest = PythonVersionRequest {
    kind: Some(Cow::Borrowed("cpython")),
    major: 3,
    minor: Some(10),
    patch: None,
    suffix: None,
};
const SELF_SITE_PACKAGES: &str = "python3.10/site-packages";

/// Bootstraps the venv for rye itself
pub fn ensure_self_venv(output: CommandOutput) -> Result<PathBuf, Error> {
    let app_dir = get_app_dir().context("could not get app dir")?;
    let dir = app_dir.join("self");
    if dir.is_dir() {
        return Ok(dir);
    }

    if output != CommandOutput::Quiet {
        eprintln!("Bootstrapping rye internals");
    }

    let version = fetch(&SELF_PYTHON_VERSION, output).with_context(|| {
        format!(
            "failed to fetch internal cpython toolchain {}",
            SELF_PYTHON_VERSION
        )
    })?;
    let py_bin = get_py_bin(&version)?;

    // initialize the virtualenv
    let mut venv_cmd = Command::new(&py_bin);
    venv_cmd.arg("-mvenv");
    venv_cmd.arg("--upgrade-deps");
    venv_cmd.arg(&dir);

    let status = venv_cmd
        .status()
        .with_context(|| format!("unable to create self venv using {}", py_bin.display()))?;
    if !status.success() {
        bail!("failed to initialize virtualenv in {}", dir.display());
    }

    // upgrade pip
    if output != CommandOutput::Quiet {
        eprintln!("Upgrading pip");
    }
    let mut pip_install_cmd = Command::new(dir.join("bin/pip"));
    pip_install_cmd.arg("install");
    pip_install_cmd.arg("--upgrade");
    pip_install_cmd.arg("pip");
    if output == CommandOutput::Verbose {
        pip_install_cmd.arg("--verbose");
    } else {
        pip_install_cmd.arg("--quiet");
        pip_install_cmd.env("PYTHONWARNINGS", "ignore");
    }
    let status = pip_install_cmd
        .status()
        .context("unable to self-upgrade pip")?;
    if !status.success() {
        bail!("failed to initialize virtualenv (upgrade pip)");
    }

    // install virtualenv and unearth
    let mut pip_install_cmd = Command::new(dir.join("bin/pip"));
    pip_install_cmd.arg("install");
    pip_install_cmd.arg("virtualenv");
    pip_install_cmd.arg("unearth");
    pip_install_cmd.arg("pip-tools");
    pip_install_cmd.arg("build");
    if output != CommandOutput::Quiet {
        eprintln!("Installing internal dependencies");
    }
    if output == CommandOutput::Verbose {
        pip_install_cmd.arg("--verbose");
    } else {
        pip_install_cmd.arg("--quiet");
        pip_install_cmd.env("PYTHONWARNINGS", "ignore");
    }
    let status = pip_install_cmd
        .status()
        .context("unable to install self-dependencies")?;
    if !status.success() {
        bail!("failed to initialize virtualenv (install dependencies)");
    }

    // create shims
    let shims = app_dir.join("shims");
    fs::remove_dir_all(&shims).ok();
    fs::create_dir_all(&shims).context("tried to crate shim folder")?;
    let this = env::current_exe()?;

    // on linux symlinks cause us to mis-detect the shim because
    // it points to the actual executable.  So there we need to do
    // hardlinks instead.
    #[cfg(target_os = "linux")]
    {
        fs::hard_link(&this, shims.join("python")).context("tried to hard-link python shim")?;
        fs::hard_link(&this, shims.join("python3")).context("tried to hard-link python3 shim")?;
    }
    #[cfg(not(target_os = "linux"))]
    {
        use std::os::unix::fs::symlink;
        symlink(&this, shims.join("python")).context("tried to symlink python shim")?;
        symlink(&this, shims.join("python3")).context("tried to symlink python3 shim")?;
    }

    Ok(dir)
}

/// Returns the pip runner for the self venv
pub fn get_pip_runner(venv: &Path) -> PathBuf {
    get_pip_module(venv).join("__pip-runner__.py")
}

/// Returns the pip module for the self venv
pub fn get_pip_module(venv: &Path) -> PathBuf {
    let mut rv = venv.to_path_buf();
    rv.push("lib");
    rv.push(SELF_SITE_PACKAGES);
    rv.push("pip");
    rv
}

/// Fetches a version if missing.
pub fn fetch(
    version: &PythonVersionRequest,
    output: CommandOutput,
) -> Result<PythonVersion, Error> {
    if let Ok(version) = PythonVersion::try_from(version.clone()) {
        let py_bin = get_py_bin(&version)?;
        if py_bin.is_file() {
            if output == CommandOutput::Verbose {
                eprintln!("Python version already downloaded. Skipping.");
            }
            return Ok(version);
        }
    }

    let (version, url) = match get_download_url(version, OS, ARCH) {
        Some(result) => result,
        None => bail!("unknown version {}", version),
    };

    let target_dir = get_canonical_py_path(&version)?;
    let target_py_bin = get_py_bin(&version)?;
    if output == CommandOutput::Verbose {
        eprintln!("target dir: {}", target_dir.display());
    }
    if target_dir.is_dir() && target_py_bin.is_file() {
        if output == CommandOutput::Verbose {
            eprintln!("Python version already downloaded. Skipping.");
        }
        return Ok(version);
    }

    fs::create_dir_all(&target_dir)
        .with_context(|| format!("failed to create target folder {}", target_dir.display()))?;

    let mut archive_buffer = Vec::new();

    if output == CommandOutput::Verbose {
        eprintln!("download url: {}", url);
    }
    if output != CommandOutput::Quiet {
        eprintln!("{} {}", style("Downloading").cyan(), version);
    }

    let mut handle = curl::easy::Easy::new();
    handle.url(url)?;
    handle.progress(true)?;
    handle.follow_location(true)?;

    let write_archive = &mut archive_buffer;
    {
        let mut transfer = handle.transfer();
        let mut pb = None;
        transfer.progress_function(move |a, b, _, _| {
            if output == CommandOutput::Quiet {
                return true;
            }

            let (down_len, down_pos) = (a as u64, b as u64);
            if down_len > 0 {
                if down_pos < down_len {
                    if pb.is_none() {
                        let pb_config = ProgressBar::new(down_len);
                        pb_config.set_style(
                            ProgressStyle::with_template("{wide_bar} {bytes:>7}/{total_bytes:7}")
                                .unwrap(),
                        );
                        pb = Some(pb_config);
                    }
                    pb.as_ref().unwrap().set_position(down_pos);
                } else if pb.is_some() {
                    pb.take().unwrap().finish_and_clear();
                }
            }
            true
        })?;
        transfer.write_function(move |data| {
            write_archive.write_all(data).unwrap();
            Ok(data.len())
        })?;
        transfer
            .perform()
            .with_context(|| format!("download of {} failed", &url))?;
    }

    unpack_tarball(&archive_buffer, &target_dir, 1)
        .with_context(|| format!("unpacking of downloaded tarball {} failed", &url))?;

    if output != CommandOutput::Quiet {
        eprintln!("{} Downloaded {}", style("success:").green(), version);
    }

    Ok(version)
}
