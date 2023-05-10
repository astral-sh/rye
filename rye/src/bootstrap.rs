use std::borrow::Cow;
use std::env::consts::{ARCH, OS};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{self, AtomicBool};
use std::{env, fs};

use anyhow::{bail, Context, Error};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;

use crate::config::{get_app_dir, get_canonical_py_path, get_toolchain_python_bin};
use crate::consts::VENV_BIN;
use crate::sources::{get_download_url, PythonVersion, PythonVersionRequest};
use crate::utils::{symlink_file, unpack_tarball, CommandOutput};

pub const SELF_PYTHON_VERSION: PythonVersionRequest = PythonVersionRequest {
    kind: Some(Cow::Borrowed("cpython")),
    major: 3,
    minor: Some(10),
    patch: None,
    suffix: None,
};
const SELF_VERSION: u64 = 1;

#[cfg(unix)]
const SELF_SITE_PACKAGES: &str = "python3.10/site-packages";
#[cfg(windows)]
const SELF_SITE_PACKAGES: &str = "site-packages";

const SELF_REQUIREMENTS: &str = r#"
build==0.10.0
certifi==2022.12.7
charset-normalizer==3.1.0
click==8.1.3
distlib==0.3.6
filelock==3.12.0
idna==3.4
packaging==23.1
pip-tools==6.13.0
platformdirs==3.4.0
pyproject_hooks==1.0.0
requests==2.29.0
tomli==2.0.1
unearth==0.9.0
urllib3==1.26.15
virtualenv==20.22.0
"#;

static FORCED_TO_UPDATE: AtomicBool = AtomicBool::new(false);

fn is_up_to_date() -> bool {
    static UP_TO_UPDATE: Lazy<bool> = Lazy::new(|| match get_app_dir() {
        Ok(dir) => fs::read_to_string(dir.join("self").join("tool-version.txt"))
            .ok()
            .map_or(false, |x| x.parse() == Ok(SELF_VERSION)),
        Err(_) => false,
    });
    *UP_TO_UPDATE || FORCED_TO_UPDATE.load(atomic::Ordering::Relaxed)
}

/// Bootstraps the venv for rye itself
pub fn ensure_self_venv(output: CommandOutput) -> Result<PathBuf, Error> {
    let app_dir = get_app_dir().context("could not get app dir")?;
    let venv_dir = app_dir.join("self");

    if venv_dir.is_dir() {
        if is_up_to_date() {
            return Ok(venv_dir);
        } else {
            if output != CommandOutput::Quiet {
                eprintln!("detected outdated rye internals. Refreshing");
            }
            fs::remove_dir_all(&venv_dir).context("could not remove self-venv for update")?;
        }
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
    let py_bin = get_toolchain_python_bin(&version)?;

    // initialize the virtualenv
    let mut venv_cmd = Command::new(&py_bin);
    venv_cmd.arg("-mvenv");
    venv_cmd.arg("--upgrade-deps");
    venv_cmd.arg(&venv_dir);

    let status = venv_cmd
        .status()
        .with_context(|| format!("unable to create self venv using {}", py_bin.display()))?;
    if !status.success() {
        bail!("failed to initialize virtualenv in {}", venv_dir.display());
    }

    do_update(output, &venv_dir, app_dir)?;

    fs::write(venv_dir.join("tool-version.txt"), SELF_VERSION.to_string())?;
    FORCED_TO_UPDATE.store(true, atomic::Ordering::Relaxed);

    Ok(venv_dir)
}

fn do_update(output: CommandOutput, venv_dir: &Path, app_dir: &Path) -> Result<(), Error> {
    if output != CommandOutput::Quiet {
        eprintln!("Upgrading pip");
    }
    let venv_bin = venv_dir.join(VENV_BIN);

    let mut pip_install_cmd = Command::new(venv_bin.join("pip"));
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
    let mut req_file = NamedTempFile::new()?;
    writeln!(req_file, "{}", SELF_REQUIREMENTS)?;
    let mut pip_install_cmd = Command::new(venv_bin.join("pip"));
    pip_install_cmd
        .arg("install")
        .arg("-r")
        .arg(req_file.path());
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
    let shims = app_dir.join("shims");
    fs::remove_dir_all(&shims).ok();
    fs::create_dir_all(&shims).context("tried to create shim folder")?;
    let this = env::current_exe()?;
    #[cfg(unix)]
    {
        let use_softlinks = !cfg!(target_os = "linux");
        if use_softlinks || fs::hard_link(&this, shims.join("python")).is_err() {
            symlink_file(&this, shims.join("python")).context("tried to symlink python shim")?;
        }
        if use_softlinks || fs::hard_link(&this, shims.join("python3")).is_err() {
            symlink_file(&this, shims.join("python3")).context("tried to symlink python3 shim")?;
        }
    }
    #[cfg(windows)]
    {
        symlink_file(this, shims.join("python.exe")).context("tried to symlink python shim")?;
    }

    Ok(())
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

fn check_hash(content: &[u8], hash: &'static str) -> Result<(), Error> {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let digest = hasher.finalize();
    let digest = hex::encode(digest);
    if digest != hash {
        bail!("hash mismatch: expected {} got {}", hash, digest);
    }
    Ok(())
}

/// Fetches a version if missing.
pub fn fetch(
    version: &PythonVersionRequest,
    output: CommandOutput,
) -> Result<PythonVersion, Error> {
    if let Ok(version) = PythonVersion::try_from(version.clone()) {
        let py_bin = get_toolchain_python_bin(&version)?;
        if py_bin.is_file() {
            if output == CommandOutput::Verbose {
                eprintln!("Python version already downloaded. Skipping.");
            }
            return Ok(version);
        }
    }

    let (version, url, sha256) = match get_download_url(version, OS, ARCH) {
        Some(result) => result,
        None => bail!("unknown version {}", version),
    };

    let target_dir = get_canonical_py_path(&version)?;
    let target_py_bin = get_toolchain_python_bin(&version)?;
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

    if let Some(sha256) = sha256 {
        if output != CommandOutput::Quiet {
            eprintln!("{}", style("Checking hash").cyan());
        }
        check_hash(&archive_buffer, sha256)
            .with_context(|| format!("hash check of {} failed", &url))?;
    } else if output != CommandOutput::Quiet {
        eprintln!("hash check skipped (no hash available)");
    }

    unpack_tarball(&archive_buffer, &target_dir, 1)
        .with_context(|| format!("unpacking of downloaded tarball {} failed", &url))?;

    if output != CommandOutput::Quiet {
        eprintln!("{} Downloaded {}", style("success:").green(), version);
    }

    Ok(version)
}
