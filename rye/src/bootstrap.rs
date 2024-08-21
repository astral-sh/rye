use std::borrow::Cow;
use std::env::consts::EXE_EXTENSION;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{self, AtomicBool};
use std::{env, fs};

use anyhow::{anyhow, bail, Context, Error};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use tempfile::tempdir_in;

use crate::config::Config;
use crate::platform::{
    get_app_dir, get_canonical_py_path, get_python_bin_within, get_toolchain_python_bin,
    list_known_toolchains,
};
use crate::pyproject::latest_available_python_version;
use crate::sources::py::{get_download_url, PythonVersion, PythonVersionRequest};
use crate::utils::{check_checksum, symlink_file, unpack_archive, CommandOutput, IoPathContext};
use crate::uv::UvBuilder;

/// this is the target version that we want to fetch
pub const SELF_PYTHON_TARGET_VERSION: PythonVersionRequest = PythonVersionRequest {
    name: Some(Cow::Borrowed("cpython")),
    arch: None,
    os: None,
    major: 3,
    minor: Some(12),
    patch: None,
    suffix: None,
};

const SELF_VERSION: u64 = 22;

pub const SELF_REQUIREMENTS: &str = r#"
build==1.2.1
certifi==2024.2.2
charset-normalizer==3.3.2
click==8.1.7
distlib==0.3.8
filelock==3.12.2
idna==3.4
packaging==23.1
platformdirs==4.0.0
pyproject_hooks==1.0.0
requests==2.31.0
tomli==2.0.1
twine==5.1.1
unearth==0.14.0
urllib3==2.0.7
virtualenv==20.25.0
ruff==0.5.4
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

#[derive(Debug, Clone)]
pub(crate) enum SelfVenvStatus {
    NotUpToDate,
    DoesNotExist,
}

/// Get self venv path and check if it exists and is up to date
pub fn get_self_venv_status() -> Result<PathBuf, (PathBuf, SelfVenvStatus)> {
    let app_dir = get_app_dir();
    let venv_dir = app_dir.join("self");

    if venv_dir.is_dir() {
        if is_up_to_date() {
            Ok(venv_dir)
        } else {
            Err((venv_dir, SelfVenvStatus::NotUpToDate))
        }
    } else {
        Err((venv_dir, SelfVenvStatus::DoesNotExist))
    }
}

/// Bootstraps the venv for rye itself
pub fn ensure_self_venv(output: CommandOutput) -> Result<PathBuf, Error> {
    ensure_self_venv_with_toolchain(output, None)
}

/// Bootstraps the venv for rye itself
pub fn ensure_self_venv_with_toolchain(
    output: CommandOutput,
    toolchain_version_request: Option<PythonVersionRequest>,
) -> Result<PathBuf, Error> {
    let app_dir = get_app_dir();

    let venv_dir = match get_self_venv_status() {
        Ok(venv_dir) => return Ok(venv_dir),
        Err((venv_dir, SelfVenvStatus::DoesNotExist)) => venv_dir,
        Err((venv_dir, SelfVenvStatus::NotUpToDate)) => {
            echo!(if output, "Detected outdated rye internals. Refreshing");
            fs::remove_dir_all(&venv_dir)
                .path_context(&venv_dir, "could not remove self-venv for update")?;

            let pip_tools_dir = app_dir.join("pip-tools");
            if pip_tools_dir.is_dir() {
                fs::remove_dir_all(&pip_tools_dir)
                    .context("could not remove pip-tools for update")?;
            }

            venv_dir
        }
    };

    echo!(if output, "Bootstrapping rye internals");

    // Ensure we have uv
    let uv = UvBuilder::new()
        .with_output(CommandOutput::Quiet)
        .ensure_exists()?;

    let version = match toolchain_version_request {
        Some(ref version_request) => ensure_specific_self_toolchain(output, version_request)
            .with_context(|| {
                format!(
                    "failed to provision internal cpython toolchain {}",
                    version_request
                )
            })?,
        None => ensure_latest_self_toolchain(output).with_context(|| {
            format!(
                "failed to fetch internal cpython toolchain {}",
                SELF_PYTHON_TARGET_VERSION
            )
        })?,
    };

    let py_bin = get_toolchain_python_bin(&version)?;

    // linux specific detection of shared libraries.
    #[cfg(target_os = "linux")]
    {
        validate_shared_libraries(&py_bin)?;
    }

    // initialize the virtualenv
    {
        let uv_venv = uv.venv(&venv_dir, &py_bin, &version, None)?;
        // write our marker
        uv_venv.write_marker()?;
        // update our requirements
        uv_venv.update_requirements(SELF_REQUIREMENTS)?;

        // Update the shims
        let shims = app_dir.join("shims");
        if !shims.is_dir() {
            fs::create_dir_all(&shims).path_context(&shims, "tried to create shim folder")?;
        }

        // if rye is itself installed into the shims folder, we want to
        // use that.  Otherwise we fall back to the current executable
        let mut this = shims.join("rye").with_extension(EXE_EXTENSION);
        if !this.is_file() {
            this = env::current_exe()?;
        }

        update_core_shims(&shims, &this)?;

        uv_venv.write_tool_version(SELF_VERSION)?;
    }

    FORCED_TO_UPDATE.store(true, atomic::Ordering::Relaxed);

    Ok(venv_dir)
}

pub fn update_core_shims(shims: &Path, this: &Path) -> Result<(), Error> {
    #[cfg(unix)]
    {
        let py_shim = shims.join("python");
        let py3_shim = shims.join("python3");

        // on linux we cannot symlink at all, as this will misreport.  We will try to do
        // hardlinks and if that fails, we fall back to copying the entire file over.  This
        // for instance is needed when the rye executable is placed on a different volume
        // than ~/.rye/shims
        if cfg!(target_os = "linux") {
            fs::remove_file(&py_shim).ok();
            if fs::hard_link(this, &py_shim).is_err() {
                fs::copy(this, &py_shim).path_context(&py_shim, "tried to copy python shim")?;
            }
            fs::remove_file(&py3_shim).ok();
            if fs::hard_link(this, &py3_shim).is_err() {
                fs::copy(this, &py3_shim).path_context(&py_shim, "tried to copy python3 shim")?;
            }

        // on other unices we always use symlinks
        } else {
            fs::remove_file(&py_shim).ok();
            symlink_file(this, &py_shim).path_context(&py_shim, "tried to symlink python shim")?;
            fs::remove_file(&py3_shim).ok();
            symlink_file(this, &py3_shim)
                .path_context(&py3_shim, "tried to symlink python3 shim")?;
        }
    }

    #[cfg(windows)]
    {
        let py_shim = shims.join("python.exe");
        let pyw_shim = shims.join("pythonw.exe");
        let py3_shim = shims.join("python3.exe");

        // on windows we need privileges to symlink.  Not everyone might have that, so we
        // fall back to hardlinks.
        fs::remove_file(&py_shim).ok();
        if symlink_file(this, &py_shim).is_err() {
            fs::hard_link(this, &py_shim).path_context(&py_shim, "tried to symlink python shim")?;
        }
        fs::remove_file(&py3_shim).ok();
        if symlink_file(this, &py3_shim).is_err() {
            fs::hard_link(this, &py3_shim)
                .path_context(&py3_shim, "tried to symlink python3 shim")?;
        }
        fs::remove_file(&pyw_shim).ok();
        if symlink_file(this, &pyw_shim).is_err() {
            fs::hard_link(this, &pyw_shim)
                .path_context(&pyw_shim, "tried to symlink pythonw shim")?;
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

/// we only support cpython 3.9 to 3.12
pub fn is_self_compatible_toolchain(version: &PythonVersion) -> bool {
    version.name == "cpython" && version.major == 3 && version.minor >= 9 && version.minor <= 12
}

/// Ensure that the toolchain for the self environment is available.
fn ensure_latest_self_toolchain(output: CommandOutput) -> Result<PythonVersion, Error> {
    if let Some(version) = list_known_toolchains()?
        .into_iter()
        .map(|x| x.0)
        .filter(is_self_compatible_toolchain)
        .collect::<Vec<_>>()
        .into_iter()
        .max()
    {
        echo!(
            if output,
            "Found a compatible Python version: {}",
            style(&version).cyan()
        );
        Ok(version)
    } else {
        fetch(
            &SELF_PYTHON_TARGET_VERSION,
            FetchOptions::with_output(output),
        )
    }
}

/// Ensure a specific toolchain is available.
fn ensure_specific_self_toolchain(
    output: CommandOutput,
    toolchain_version_request: &PythonVersionRequest,
) -> Result<PythonVersion, Error> {
    let toolchain_version = latest_available_python_version(toolchain_version_request)
        .ok_or_else(|| anyhow!("requested toolchain version is not available"))?;
    if !is_self_compatible_toolchain(&toolchain_version) {
        bail!(
            "the requested toolchain version ({}) is not supported for rye-internal usage",
            toolchain_version
        );
    }
    if !get_toolchain_python_bin(&toolchain_version)?.is_file() {
        echo!(
            if output,
            "Fetching requested internal toolchain '{}'",
            toolchain_version
        );
        fetch(&toolchain_version.into(), FetchOptions::with_output(output))
    } else {
        echo!(
            if output,
            "Found a compatible Python version: {}",
            style(&toolchain_version).cyan()
        );
        Ok(toolchain_version)
    }
}

/// Fetches a python installer.
pub struct FetchOptions {
    /// How verbose should the sync be?
    pub output: CommandOutput,
    /// Forces re-downloads even if they are already there.
    pub force: bool,
    /// Causes a fetch into a non standard location.
    pub target_path: Option<PathBuf>,
    /// Include build info (overrides configured default).
    pub build_info: Option<bool>,
}

impl FetchOptions {
    /// Basic fetch options.
    pub fn with_output(output: CommandOutput) -> FetchOptions {
        FetchOptions {
            output,
            ..Default::default()
        }
    }
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            output: CommandOutput::Normal,
            force: false,
            target_path: None,
            build_info: None,
        }
    }
}

/// Fetches a version if missing.
pub fn fetch(
    version: &PythonVersionRequest,
    options: FetchOptions,
) -> Result<PythonVersion, Error> {
    // Check if there is registered toolchain that matches the request
    if options.target_path.is_none() {
        if let Ok(version) = PythonVersion::try_from(version.clone()) {
            let py_bin = get_toolchain_python_bin(&version)?;
            if !options.force && py_bin.is_file() {
                echo!(if verbose options.output, "Python version already downloaded. Skipping.");
                return Ok(version);
            }
        }
    }
    let (version, url, sha256) = match get_download_url(version) {
        Some(result) => result,
        None => bail!("unknown version {}", version),
    };

    let target_dir = match options.target_path {
        Some(ref target_dir) => {
            if target_dir.is_file() {
                bail!("target directory '{}' is a file", target_dir.display());
            }
            echo!(if options.output, "Downloading to '{}'", target_dir.display());
            if target_dir.is_dir() {
                if options.force {
                    // Refuse to remove the target directory if it's not empty and not a python installation
                    if target_dir.read_dir()?.next().is_some()
                        && !get_python_bin_within(target_dir).exists()
                    {
                        bail!(
                            "target directory '{}' exists and is not a Python installation",
                            target_dir.display()
                        );
                    }
                    fs::remove_dir_all(target_dir)
                        .path_context(target_dir, "could not remove target directory")?;
                } else {
                    bail!("target directory '{}' exists", target_dir.display());
                }
            }
            Cow::Borrowed(target_dir.as_path())
        }
        None => {
            let target_dir = get_canonical_py_path(&version)?;
            let target_py_bin = get_toolchain_python_bin(&version)?;
            if target_py_bin.is_file() {
                if !options.force {
                    echo!(if verbose options.output, "Python version already downloaded. Skipping.");
                    return Ok(version);
                }
                echo!(if options.output, "Removing the existing Python version");
                fs::remove_dir_all(&target_dir).with_context(|| {
                    format!("failed to remove target folder {}", target_dir.display())
                })?;
            }
            echo!(if verbose options.output, "target dir: {}", target_dir.display());
            Cow::Owned(target_dir)
        }
    };

    echo!(if verbose options.output, "download url: {}", url);
    echo!(if options.output, "{} {}", style("Downloading").cyan(), version);
    let archive_buffer = download_url(url, options.output)?;

    if let Some(sha256) = sha256 {
        echo!(if options.output, "{} {}", style("Checking").cyan(), "checksum");
        check_checksum(&archive_buffer, sha256)
            .with_context(|| format!("Checksum check of {} failed", &url))?;
    } else {
        echo!(if options.output, "Checksum check skipped (no hash available)");
    }

    echo!(if options.output, "{}", style("Unpacking").cyan());

    let parent = target_dir
        .parent()
        .ok_or_else(|| anyhow!("cannot unpack to root"))?;
    if !parent.exists() {
        fs::create_dir_all(parent).path_context(&target_dir, "failed to create target folder")?;
    }

    let with_build_info = options
        .build_info
        .unwrap_or_else(|| Config::current().fetch_with_build_info());
    let temp_dir = tempdir_in(parent).context("temporary unpack location")?;

    unpack_archive(&archive_buffer, temp_dir.path(), 1).with_context(|| {
        format!(
            "unpacking of downloaded tarball {} to '{}' failed",
            &url,
            temp_dir.path().display(),
        )
    })?;

    // if we want to retain build infos or the installation has no build infos, then move
    // the folder into the permanent location
    if with_build_info || !installation_has_build_info(temp_dir.path()) {
        let temp_dir = temp_dir.into_path();
        fs::rename(&temp_dir, &target_dir).map_err(|err| {
            fs::remove_dir_all(&temp_dir).ok();
            err
        })

    // otherwise move the contents of the `install` folder over.
    } else {
        fs::rename(temp_dir.path().join("install"), &target_dir)
    }
    .path_context(&target_dir, "unable to persist download")?;

    echo!(if options.output, "{} {}", style("Downloaded").green(), version);

    Ok(version)
}

fn installation_has_build_info(p: &Path) -> bool {
    let mut has_install = false;
    let mut has_build = false;
    if let Ok(dir) = p.read_dir() {
        for entry in dir.flatten() {
            match entry.file_name().to_str() {
                Some("install") => has_install = true,
                Some("build") => has_build = true,
                _ => {}
            }
        }
    }
    has_install && has_build
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

    // on windows we want to disable revocation checks.  The reason is that MITM proxies
    // will otherwise not work.  This is a schannel specific behavior anyways.
    // for more information see https://github.com/curl/curl/issues/264
    #[cfg(windows)]
    {
        handle.ssl_options(curl::easy::SslOpt::new().no_revoke(true))?;
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
    use std::process::Command;
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
        Visit https://rye.astral.sh/guide/faq/#missing-shared-libraries-on-linux for next steps."
    );
}
