use std::borrow::Cow;
use std::env::consts::{ARCH, EXE_EXTENSION, OS};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{self, AtomicBool};
use std::{env, fs};

use anyhow::{bail, Context, Error};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use tempfile::NamedTempFile;

use crate::config::Config;
use crate::consts::VENV_BIN;
use crate::platform::{
    get_app_dir, get_canonical_py_path, get_toolchain_python_bin, list_known_toolchains,
    symlinks_supported,
};
use crate::sources::{get_download_url, PythonVersion, PythonVersionRequest};
use crate::utils::{
    check_checksum, set_proxy_variables, symlink_file, unpack_archive, CommandOutput,
};

/// this is the target version that we want to fetch
pub const SELF_PYTHON_TARGET_VERSION: PythonVersionRequest = PythonVersionRequest {
    kind: Some(Cow::Borrowed("cpython")),
    major: 3,
    minor: Some(11),
    patch: None,
    suffix: None,
};

const SELF_VERSION: u64 = 3;

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
twine==4.0.2
unearth==0.9.0
urllib3==1.26.15
virtualenv==20.22.0
"#;

static FORCED_TO_UPDATE: AtomicBool = AtomicBool::new(false);

fn is_up_to_date() -> bool {
    static UP_TO_UPDATE: Lazy<bool> = Lazy::new(|| {
        fs::read_to_string(get_app_dir().join("self").join("tool-version.txt"))
            .ok()
            .map_or(false, |x| x.parse() == Ok(SELF_VERSION))
    });
    *UP_TO_UPDATE || FORCED_TO_UPDATE.load(atomic::Ordering::Relaxed)
}

/// Bootstraps the venv for rye itself
pub fn ensure_self_venv(output: CommandOutput) -> Result<PathBuf, Error> {
    let app_dir = get_app_dir();
    let venv_dir = app_dir.join("self");
    let pip_tools_dir = app_dir.join("pip-tools");

    if venv_dir.is_dir() {
        if is_up_to_date() {
            return Ok(venv_dir);
        } else {
            if output != CommandOutput::Quiet {
                echo!("detected outdated rye internals. Refreshing");
            }
            fs::remove_dir_all(&venv_dir).context("could not remove self-venv for update")?;
            if pip_tools_dir.is_dir() {
                fs::remove_dir_all(&pip_tools_dir)
                    .context("could not remove pip-tools for update")?;
            }
        }
    }

    if output != CommandOutput::Quiet {
        echo!("Bootstrapping rye internals");
    }

    let version = ensure_self_toolchain(output).with_context(|| {
        format!(
            "failed to fetch internal cpython toolchain {}",
            SELF_PYTHON_TARGET_VERSION
        )
    })?;
    let py_bin = get_toolchain_python_bin(&version)?;

    // linux specific detection of shared libraries.
    #[cfg(target_os = "linux")]
    {
        validate_shared_libraries(&py_bin)?;
    }

    // initialize the virtualenv
    let mut venv_cmd = Command::new(&py_bin);
    venv_cmd.arg("-mvenv");
    venv_cmd.arg("--upgrade-deps");

    // unlike virtualenv which we use after bootstrapping, the stdlib python
    // venv does not detect symlink support itself and needs to be coerced into
    // when available.
    if cfg!(windows) && symlinks_supported() {
        venv_cmd.arg("--symlinks");
    }

    venv_cmd.arg(&venv_dir);
    set_proxy_variables(&mut venv_cmd);

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
        echo!("Upgrading pip");
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
        echo!("Installing internal dependencies");
    }
    if output == CommandOutput::Verbose {
        pip_install_cmd.arg("--verbose");
    } else {
        pip_install_cmd.arg("--quiet");
        pip_install_cmd.env("PYTHONWARNINGS", "ignore");
    }
    set_proxy_variables(&mut pip_install_cmd);
    let status = pip_install_cmd
        .status()
        .context("unable to install self-dependencies")?;
    if !status.success() {
        bail!("failed to initialize virtualenv (install dependencies)");
    }
    let shims = app_dir.join("shims");
    if !shims.is_dir() {
        fs::create_dir_all(&shims).context("tried to create shim folder")?;
    }

    // if rye is itself installed into the shims folder, we want to
    // use that.  Otherwise we fall back to the current executable
    let mut this = shims.join("rye").with_extension(EXE_EXTENSION);
    if !this.is_file() {
        this = env::current_exe()?;
    }

    update_core_shims(&shims, &this)?;

    Ok(())
}

pub fn update_core_shims(shims: &Path, this: &Path) -> Result<(), Error> {
    #[cfg(unix)]
    {
        let use_softlinks = !cfg!(target_os = "linux");
        fs::remove_file(shims.join("python")).ok();
        if use_softlinks || fs::hard_link(this, shims.join("python")).is_err() {
            symlink_file(this, shims.join("python")).context("tried to symlink python shim")?;
        }
        fs::remove_file(shims.join("python3")).ok();
        if use_softlinks || fs::hard_link(this, shims.join("python3")).is_err() {
            symlink_file(this, shims.join("python3")).context("tried to symlink python3 shim")?;
        }
    }

    #[cfg(windows)]
    {
        // on windows we need privileges to symlink.  Not everyone might have that, so we
        // fall back to hardlinks.
        fs::remove_file(shims.join("python.exe")).ok();
        if symlink_file(this, shims.join("python.exe")).is_err() {
            fs::hard_link(this, shims.join("python.exe"))
                .context("tried to symlink python shim")?;
        }
        fs::remove_file(shims.join("pythonw.exe")).ok();
        if symlink_file(this, shims.join("pythonw.exe")).is_err() {
            fs::hard_link(this, shims.join("pythonw.exe"))
                .context("tried to symlink pythonw shim")?;
        }
    }

    Ok(())
}

/// Returns the pip runner for the self venv
pub fn get_pip_runner(venv: &Path) -> Result<PathBuf, Error> {
    Ok(get_pip_module(venv)?.join("__pip-runner__.py"))
}

/// Returns the pip module for the self venv
pub fn get_pip_module(venv: &Path) -> Result<PathBuf, Error> {
    let mut rv = venv.to_path_buf();
    rv.push("lib");
    #[cfg(windows)]
    {
        rv.push("site-packages");
    }
    #[cfg(unix)]
    {
        // This is not optimal.  We find the first thing that
        // looks like pythonX.X/site-packages and just use it.
        // It also means that this requires us to do some unnecessary
        // file system operations.  However given how hopefully
        // infrequent this function is called, we might be good.
        let dir = rv.read_dir()?;
        let mut found = false;
        for entry in dir.filter_map(|x| x.ok()) {
            let filename = entry.file_name();
            if let Some(filename) = filename.to_str() {
                if filename.starts_with("python") {
                    rv.push(filename);
                    rv.push("site-packages");
                    if rv.is_dir() {
                        found = true;
                        break;
                    } else {
                        rv.pop();
                        rv.pop();
                    }
                }
            }
        }
        if !found {
            bail!("no site-packages in venv");
        }
    }
    rv.push("pip");
    Ok(rv)
}

/// we only support cpython 3.9 to 3.11
pub fn is_self_compatible_toolchain(version: &PythonVersion) -> bool {
    version.kind == "cpython" && version.major == 3 && version.minor >= 9 && version.minor < 12
}

fn ensure_self_toolchain(output: CommandOutput) -> Result<PythonVersion, Error> {
    let possible_versions = list_known_toolchains()?
        .into_iter()
        .map(|x| x.0)
        .filter(is_self_compatible_toolchain)
        .collect::<Vec<_>>();

    if let Some(version) = possible_versions.into_iter().max() {
        echo!(
            "Found a compatible python version: {}",
            style(&version).cyan()
        );
        Ok(version)
    } else {
        fetch(&SELF_PYTHON_TARGET_VERSION, output)
    }
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
                echo!("Python version already downloaded. Skipping.");
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
        echo!("target dir: {}", target_dir.display());
    }
    if target_dir.is_dir() && target_py_bin.is_file() {
        if output == CommandOutput::Verbose {
            echo!("Python version already downloaded. Skipping.");
        }
        return Ok(version);
    }

    fs::create_dir_all(&target_dir)
        .with_context(|| format!("failed to create target folder {}", target_dir.display()))?;

    if output == CommandOutput::Verbose {
        echo!("download url: {}", url);
    }
    if output != CommandOutput::Quiet {
        echo!("{} {}", style("Downloading").cyan(), version);
    }
    let archive_buffer = download_url(url, output)?;

    if let Some(sha256) = sha256 {
        if output != CommandOutput::Quiet {
            echo!("{}", style("Checking checksum").cyan());
        }
        check_checksum(&archive_buffer, sha256)
            .with_context(|| format!("hash check of {} failed", &url))?;
    } else if output != CommandOutput::Quiet {
        echo!("Checksum check skipped (no hash available)");
    }

    unpack_archive(&archive_buffer, &target_dir, 1)
        .with_context(|| format!("unpacking of downloaded tarball {} failed", &url))?;

    if output != CommandOutput::Quiet {
        echo!("{} Downloaded {}", style("success:").green(), version);
    }

    Ok(version)
}

pub fn download_url(url: &str, output: CommandOutput) -> Result<Vec<u8>, Error> {
    match download_url_ignore_404(url, output)? {
        Some(result) => Ok(result),
        None => bail!("Failed to download: 404 not found"),
    }
}

pub fn download_url_ignore_404(url: &str, output: CommandOutput) -> Result<Option<Vec<u8>>, Error> {
    // for now we only allow HTTPS downloads.
    if !url.starts_with("https://") {
        bail!("Refusing insecure download");
    }

    let config = Config::current();
    let mut archive_buffer = Vec::new();
    let mut handle = curl::easy::Easy::new();
    handle.url(url)?;
    handle.progress(true)?;
    handle.follow_location(true)?;

    // we only do https requests here, so we always set an https proxy
    if let Some(proxy) = config.https_proxy_url() {
        handle.proxy(&proxy)?;
    }

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
    let code = handle.response_code()?;
    if code == 404 {
        Ok(None)
    } else if !(200..300).contains(&code) {
        bail!("Failed to download: {}", code)
    } else {
        Ok(Some(archive_buffer))
    }
}

#[cfg(target_os = "linux")]
fn validate_shared_libraries(py: &Path) -> Result<(), Error> {
    let out = Command::new("ldd")
        .arg(py)
        .output()
        .context("unable to invoke ldd on downloaded python binary")?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut missing = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if let Some((before, after)) = line.split_once(" => ") {
            if after == "not found" && !missing.contains(&before) {
                missing.push(before);
            }
        }
    }

    if missing.is_empty() {
        return Ok(());
    }

    missing.sort();
    echo!(
        "{}: detected missing shared librar{} required by Python:",
        style("error").red(),
        if missing.len() == 1 { "y" } else { "ies" }
    );
    for lib in missing {
        echo!("  - {}", style(lib).yellow());
    }
    bail!(
        "Python installation is unable to run on this machine due to missing libraries.\n\
        Visit https://rye-up.com/guide/faq/#missing-shared-libraries-on-linux for next steps."
    );
}
