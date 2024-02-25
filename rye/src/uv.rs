use crate::bootstrap::download_url;
use crate::platform::get_app_dir;
use crate::pyproject::write_venv_marker;
use crate::sources::py::PythonVersion;
use crate::sources::uv::{UvDownload, UvRequest};
use crate::utils::{
    check_checksum, set_proxy_variables, unpack_archive, update_venv_sync_marker, CommandOutput,
    IoPathContext,
};
use anyhow::{anyhow, Context, Error};
use std::fs::{self, remove_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::NamedTempFile;

// Represents a uv binary and associated functions
// to bootstrap rye using uv.
#[derive(Clone)]
pub struct Uv {
    output: CommandOutput,
    uv_bin: PathBuf,
}

impl Uv {
    /// Ensure that the uv binary is available.
    /// This will function will download the UV binary if it is not available.
    /// and bootstrap it into [RYE_HOME]/uv/[version]/uv.
    ///
    /// See [`Uv::cmd`] to get access to the uv binary in a safe way.
    ///
    /// Example:
    ///   ```rust
    ///   use rye::sources::uv::Uv;
    ///   use rye::utils::CommandOutput;
    ///   let uv = Uv::ensure_exists(CommandOutput::Normal).expect("Failed to ensure uv binary is available");
    ///   let status = uv.cmd().arg("--version").status().expect("Failed to run uv");
    ///   assert!(status.success());
    ///   ```
    pub fn ensure_exists(output: CommandOutput) -> Result<Self, Error> {
        // Request a download for the default uv binary for this platform.
        // For instance on aarch64 macos this will request a compatible uv version.
        let download = UvDownload::try_from(UvRequest::default())?;
        let base_dir = get_app_dir().join("uv");
        let uv_dir = base_dir.join(download.version());
        let uv_bin = if cfg!(windows) {
            let mut bin = uv_dir.join("uv");
            bin.set_extension("exe");
            bin
        } else {
            uv_dir.join("uv")
        };

        if uv_dir.exists() && uv_bin.is_file() {
            return Ok(Self { uv_bin, output });
        }

        Self::download(&download, &uv_dir, output)?;
        Self::cleanup_old_versions(&base_dir, &uv_dir)?;
        if uv_dir.exists() && uv_bin.is_file() {
            return Ok(Self { uv_bin, output });
        }

        Err(anyhow!("Failed to ensure uv binary is available"))
    }

    /// Remove all directories in [RYE_HOME]/uv that are not the current version.
    fn cleanup_old_versions(base_dir: &Path, current_version: &Path) -> Result<(), Error> {
        let versions = base_dir
            .read_dir()?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_dir())
            .filter(|entry| entry.path() != current_version);

        for entry in versions {
            if let Err(e) = remove_dir_all(entry.path()) {
                warn!("Failed to remove old uv version: {}", e);
            }
        }
        Ok(())
    }

    /// Downloads a uv binary and unpacks it into the given directory.
    fn download(download: &UvDownload, uv_dir: &Path, output: CommandOutput) -> Result<(), Error> {
        // Download the version
        let archive_buffer = download_url(&download.url, output)?;

        // All uv downloads must have a sha256 checksum
        check_checksum(&archive_buffer, &download.sha256)
            .with_context(|| format!("Checksum check of {} failed", download.url))?;

        // Unpack the archive once we ensured that the checksum is correct
        // The tarballs have a top level directory that we need to strip.
        // The windows zip files don't.
        let strip = if download.url.ends_with("zip") { 0 } else { 1 };

        unpack_archive(&archive_buffer, uv_dir, strip).with_context(|| {
            format!(
                "unpacking of downloaded tarball {} to '{}' failed",
                download.url,
                uv_dir.display(),
            )
        })?;

        Ok(())
    }

    /// Returns a new command with the uv binary as the command to run.
    /// The command will have the correct proxy settings and verbosity level based on CommandOutput.
    pub fn cmd(&self) -> Command {
        let mut cmd = Command::new(&self.uv_bin);

        match self.output {
            CommandOutput::Verbose => {
                cmd.arg("--verbose");
            }
            CommandOutput::Quiet => {
                cmd.arg("--quiet");
                cmd.env("PYTHONWARNINGS", "ignore");
            }
            CommandOutput::Normal => {}
        }

        set_proxy_variables(&mut cmd);
        cmd
    }

    /// Ensures a venv is exists or is created at the given path.
    /// Returns a UvWithVenv that can be used to run commands in the venv.
    pub fn venv(
        &self,
        venv_dir: &Path,
        py_bin: &Path,
        version: &PythonVersion,
        prompt: Option<&str>,
    ) -> Result<UvWithVenv, Error> {
        let mut cmd = self.cmd();
        cmd.arg("venv").arg("--python").arg(py_bin);
        if let Some(prompt) = prompt {
            cmd.arg("--prompt").arg(prompt);
        }
        cmd.arg(venv_dir);
        let status = cmd.status().with_context(|| {
            format!(
                "unable to create self venv using {}. It might be that \
                      the used Python build is incompatible with this machine. \
                      For more information see https://rye-up.com/guide/installation/",
                py_bin.display()
            )
        })?;

        if !status.success() {
            return Err(anyhow!(
                "Failed to create self venv using {}. uv exited with status: {}",
                py_bin.display(),
                status
            ));
        }
        Ok(UvWithVenv::new(self.clone(), venv_dir, version))
    }
}

// Represents a venv generated and managed by uv
pub struct UvWithVenv {
    uv: Uv,
    venv_path: PathBuf,
    py_version: PythonVersion,
}

impl UvWithVenv {
    fn new(uv: Uv, venv_dir: &Path, version: &PythonVersion) -> Self {
        UvWithVenv {
            uv,
            py_version: version.clone(),
            venv_path: venv_dir.to_path_buf(),
        }
    }

    /// Returns a new command with the uv binary as the command to run.
    /// The command will have the correct proxy settings and verbosity level based on CommandOutput.
    /// The command will also have the VIRTUAL_ENV environment variable set to the venv path.
    pub fn venv_cmd(&self) -> Command {
        let mut cmd = self.uv.cmd();
        cmd.env("VIRTUAL_ENV", &self.venv_path);
        cmd
    }

    /// Writes a rye-venv.json for this venv.
    pub fn write_marker(&self) -> Result<(), Error> {
        write_venv_marker(&self.venv_path, &self.py_version)
    }

    /// Updates the venv to the given pip version and requirements.
    pub fn update(&self, pip_version: &str, requirements: &str) -> Result<(), Error> {
        self.update_pip(pip_version)?;
        self.update_requirements(requirements)?;
        Ok(())
    }

    /// Updates the pip version in the venv.
    pub fn update_pip(&self, pip_version: &str) -> Result<(), Error> {
        self.venv_cmd()
            .arg("pip")
            .arg("install")
            .arg("--upgrade")
            .arg(pip_version)
            .status()
            .with_context(|| {
                format!(
                    "unable to update pip in venv at {}",
                    self.venv_path.display()
                )
            })?;

        Ok(())
    }

    /// Updates the requirements in the venv.
    pub fn update_requirements(&self, requirements: &str) -> Result<(), Error> {
        let mut req_file = NamedTempFile::new()?;
        writeln!(req_file, "{}", requirements)?;

        self.venv_cmd()
            .arg("pip")
            .arg("install")
            .arg("--upgrade")
            .arg("-r")
            .arg(req_file.path())
            .status()
            .with_context(|| {
                format!(
                    "unable to update requirements in venv at {}",
                    self.venv_path.display()
                )
            })?;

        Ok(())
    }

    /// Writes the tool version to the venv.
    pub fn write_tool_version(&self, version: u64) -> Result<(), Error> {
        let tool_version_path = self.venv_path.join("tool-version.txt");
        fs::write(&tool_version_path, version.to_string())
            .path_context(&tool_version_path, "could not write tool version")?;
        Ok(())
    }

    /// Update the cloud synchronization marker for the given path
    pub fn sync_marker(&self) {
        update_venv_sync_marker(self.uv.output, &self.venv_path)
    }
}
