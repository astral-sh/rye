use crate::bootstrap::{download_url, SELF_REQUIREMENTS};
use crate::lock::{make_project_root_fragment, KeyringProvider};
use crate::piptools::LATEST_PIP;
use crate::platform::get_app_dir;
use crate::pyproject::{read_venv_marker, write_venv_marker, ExpandedSources};
use crate::sources::py::PythonVersion;
use crate::sources::uv::{UvDownload, UvRequest};
use crate::utils::{
    check_checksum, set_proxy_variables, unpack_archive, update_venv_sync_marker, CommandOutput,
    IoPathContext,
};
use anyhow::{anyhow, Context, Error};
use pep508_rs::Requirement;
use std::fs::{self, remove_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

#[derive(Default)]
pub struct UvInstallOptions {
    pub importlib_workaround: bool,
    pub extras: Vec<Requirement>,
    pub refresh: bool,
    pub keyring_provider: KeyringProvider,
}

pub enum UvPackageUpgrade {
    /// Upgrade all packages.
    All,
    /// Upgrade the specific set of packages.
    Packages(Vec<String>),
    /// Upgrade nothing (default).
    Nothing,
}

struct UvCompileOptions {
    pub allow_prerelease: bool,
    pub exclude_newer: Option<String>,
    pub upgrade: UvPackageUpgrade,
    pub no_deps: bool,
    pub no_header: bool,
    pub keyring_provider: KeyringProvider,
    pub generate_hashes: bool,
    pub universal: bool,
}

impl UvCompileOptions {
    fn add_as_pip_args(self, cmd: &mut Command) {
        if self.no_header {
            cmd.arg("--no-header");
        }

        if self.no_deps {
            cmd.arg("--no-deps");
        }

        if self.generate_hashes {
            cmd.arg("--generate-hashes");
        }

        if self.allow_prerelease {
            cmd.arg("--prerelease=allow");
        }

        if let Some(dt) = self.exclude_newer {
            cmd.arg("--exclude-newer").arg(dt);
        }

        if self.universal {
            cmd.arg("--universal");
        }

        match self.upgrade {
            UvPackageUpgrade::All => {
                cmd.arg("--upgrade");
            }
            UvPackageUpgrade::Packages(ref pkgs) => {
                for pkg in pkgs {
                    cmd.arg("--upgrade-package").arg(pkg);
                }
            }
            UvPackageUpgrade::Nothing => {}
        }

        self.keyring_provider.add_as_pip_args(cmd);
    }
}

impl Default for UvCompileOptions {
    fn default() -> Self {
        Self {
            allow_prerelease: false,
            exclude_newer: None,
            upgrade: UvPackageUpgrade::Nothing,
            no_deps: false,
            no_header: false,
            generate_hashes: false,
            keyring_provider: KeyringProvider::Disabled,
            universal: false,
        }
    }
}

pub struct UvSyncOptions {
    pub keyring_provider: KeyringProvider,
}

impl UvSyncOptions {
    pub fn add_as_pip_args(self, cmd: &mut Command) {
        self.keyring_provider.add_as_pip_args(cmd);
    }
}

impl Default for UvSyncOptions {
    fn default() -> Self {
        Self {
            keyring_provider: KeyringProvider::Disabled,
        }
    }
}
pub struct UvBuilder {
    workdir: Option<PathBuf>,
    sources: Option<ExpandedSources>,
    output: CommandOutput,
}

impl UvBuilder {
    pub fn new() -> Self {
        Self {
            workdir: None,
            sources: None,
            output: CommandOutput::Normal,
        }
    }

    pub fn with_workdir(self, workdir: &Path) -> Self {
        Self {
            workdir: Some(workdir.to_path_buf()),
            ..self
        }
    }

    pub fn with_sources(self, sources: ExpandedSources) -> Self {
        Self {
            sources: Some(sources),
            ..self
        }
    }

    pub fn with_output(self, output: CommandOutput) -> Self {
        Self { output, ..self }
    }

    pub fn ensure_exists(self) -> Result<Uv, Error> {
        let workdir = self.workdir.unwrap_or(std::env::current_dir()?);
        let sources = self.sources.unwrap_or_else(ExpandedSources::empty);
        Uv::ensure(workdir, sources, self.output)
    }
}

// Represents a uv binary and associated functions
// to bootstrap rye using uv.
#[derive(Clone)]
pub struct Uv {
    output: CommandOutput,
    uv_bin: PathBuf,
    workdir: PathBuf,
    sources: ExpandedSources,
}

impl Default for Uv {
    fn default() -> Self {
        Uv {
            output: CommandOutput::Normal,
            uv_bin: PathBuf::new(),
            workdir: std::env::current_dir().unwrap_or_default(),
            sources: ExpandedSources::empty(),
        }
    }
}

impl Uv {
    /// Ensure that the uv binary is available.
    /// This will function will download the UV binary if it is not available.
    /// and bootstrap it into [RYE_HOME]/uv/[version]/uv.
    ///
    /// See [`Uv::cmd`] to get access to the uv binary in a safe way.
    fn ensure(
        workdir: PathBuf,
        sources: ExpandedSources,
        output: CommandOutput,
    ) -> Result<Self, Error> {
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
            return Ok(Uv {
                output,
                uv_bin,
                workdir,
                sources,
            });
        }

        Self::download(&download, &uv_dir, output)?;
        Self::cleanup_old_versions(&base_dir, &uv_dir)?;
        if uv_dir.exists() && uv_bin.is_file() {
            return Ok(Uv {
                output,
                uv_bin,
                workdir,
                sources,
            });
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

    /// Set the [`CommandOutput`] level for subsequent invocations of uv.
    #[must_use]
    pub fn with_output(self, output: CommandOutput) -> Self {
        Self { output, ..self }
    }

    /// Returns a new command with the uv binary as the command to run.
    /// The command will have the correct proxy settings and verbosity level based on CommandOutput.
    pub fn cmd(&self) -> Command {
        let mut cmd = Command::new(&self.uv_bin);
        cmd.current_dir(&self.workdir);
        cmd.env("PROJECT_ROOT", make_project_root_fragment(&self.workdir));

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
        match read_venv_marker(venv_dir) {
            Some(venv) if venv.is_compatible(version) => {
                Ok(UvWithVenv::new(self.clone(), venv_dir, version))
            }
            _ => self.create_venv(venv_dir, py_bin, version, prompt),
        }
    }

    /// Get uv binary path
    ///
    /// Warning: Always use self.cmd() when at all possible
    pub fn uv_bin(&self) -> &Path {
        &self.uv_bin
    }

    fn create_venv(
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
                      For more information see https://rye.astral.sh/guide/installation/",
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

    #[allow(clippy::too_many_arguments)]
    pub fn lockfile(
        &self,
        py_version: &PythonVersion,
        source: &Path,
        target: &Path,
        allow_prerelease: bool,
        exclude_newer: Option<String>,
        upgrade: UvPackageUpgrade,
        keyring_provider: KeyringProvider,
        generate_hashes: bool,
        universal: bool,
    ) -> Result<(), Error> {
        let options = UvCompileOptions {
            allow_prerelease,
            exclude_newer,
            upgrade,
            no_deps: false,
            no_header: true,
            generate_hashes,
            keyring_provider,
            universal,
        };

        let mut cmd = self.cmd();
        cmd.arg("pip").arg("compile").env_remove("VIRTUAL_ENV");

        self.sources.add_as_pip_args(&mut cmd);
        options.add_as_pip_args(&mut cmd);

        cmd.arg("--python-version")
            .arg(py_version.format_simple())
            .arg("--output-file")
            .arg(target);

        cmd.arg(source);

        let status = cmd.status().with_context(|| {
            format!(
                "Unable to run uv pip compile and generate {}",
                target.to_str().unwrap_or("<unknown>")
            )
        })?;

        if !status.success() {
            return Err(anyhow!(
                "Failed to run uv compile {}. uv exited with status: {}",
                target.to_str().unwrap_or("<unknown>"),
                status
            ));
        }
        Ok(())
    }
}

// Represents a venv generated and managed by uv
pub struct UvWithVenv {
    uv: Uv,
    venv_path: PathBuf,
    py_version: PythonVersion,
}

impl UvWithVenv {
    pub fn new(uv: Uv, venv_dir: &Path, version: &PythonVersion) -> Self {
        UvWithVenv {
            uv,
            py_version: version.clone(),
            venv_path: venv_dir.to_path_buf(),
        }
    }

    /// Returns a new command with the uv binary as the command to run.
    /// The command will have the correct proxy settings and verbosity level based on CommandOutput.
    /// The command will also have the VIRTUAL_ENV environment variable set to the venv path.
    fn venv_cmd(&self) -> Command {
        let mut cmd = self.uv.cmd();
        cmd.env("VIRTUAL_ENV", &self.venv_path);
        cmd
    }

    /// Writes a rye-venv.json for this venv.
    pub fn write_marker(&self) -> Result<(), Error> {
        write_venv_marker(&self.venv_path, &self.py_version)
    }

    /// Set the output level for subsequent invocations of uv.
    pub fn with_output(self, output: CommandOutput) -> Self {
        UvWithVenv {
            uv: Uv { output, ..self.uv },
            venv_path: self.venv_path,
            py_version: self.py_version,
        }
    }

    /// Updates the venv to the given pip version and requirements.
    pub fn update(&self, pip_version: &str, requirements: &str) -> Result<(), Error> {
        self.update_pip(pip_version)?;
        self.update_requirements(requirements)?;
        Ok(())
    }

    /// Install the bootstrap requirements in the venv.
    pub fn bootstrap(&self) -> Result<(), Error> {
        self.update(LATEST_PIP, SELF_REQUIREMENTS)?;
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

    /// Freezes the venv.
    pub fn freeze(&self) -> Result<(), Error> {
        let status = self
            .venv_cmd()
            .arg("pip")
            .arg("freeze")
            .status()
            .with_context(|| format!("unable to freeze venv at {}", self.venv_path.display()))?;

        if !status.success() {
            return Err(anyhow!(
                "Failed to freeze venv at {}. uv exited with status: {}",
                self.venv_path.display(),
                status
            ));
        }

        Ok(())
    }

    /// Installs the given requirement in the venv.
    ///
    /// If you provide a list of extras, they will be installed as well.
    /// For python 3.7 you are best off setting importlib_workaround to true.
    pub fn install(
        &self,
        requirement: &Requirement,
        options: UvInstallOptions,
    ) -> Result<(), Error> {
        let mut cmd = self.venv_cmd();

        cmd.arg("pip").arg("install");

        if options.refresh {
            cmd.arg("--refresh");
        }

        options.keyring_provider.add_as_pip_args(&mut cmd);

        self.uv.sources.add_as_pip_args(&mut cmd);

        cmd.arg("--").arg(requirement.to_string());

        for pkg in options.extras {
            cmd.arg(pkg.to_string());
        }

        // We could also include this based on the python version,
        // but we really want to leave this to the caller to decide.
        if options.importlib_workaround {
            cmd.arg("importlib-metadata==6.6.0");
        }

        let status = cmd.status().with_context(|| {
            format!(
                "unable to install {} in venv at {}",
                requirement,
                self.venv_path.display()
            )
        })?;

        if !status.success() {
            return Err(anyhow!(
                "Installation of {} failed in venv at {}. uv exited with status: {}",
                requirement,
                self.venv_path.display(),
                status
            ));
        }

        Ok(())
    }

    /// Syncs the venv
    pub fn sync(&self, lockfile: &Path, options: UvSyncOptions) -> Result<(), Error> {
        let mut cmd = self.venv_cmd();
        cmd.arg("pip").arg("sync");

        options.add_as_pip_args(&mut cmd);

        self.uv.sources.add_as_pip_args(&mut cmd);

        let status = cmd
            .arg(lockfile)
            .status()
            .with_context(|| format!("unable to run sync {}", self.venv_path.display()))?;

        if !status.success() {
            return Err(anyhow!(
                "Installation of dependencies failed in venv at {}. uv exited with status: {}",
                self.venv_path.display(),
                status
            ));
        }
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

    /// Resolves the given requirement and returns the resolved requirement.

    /// This will spawn `uv` and read from it's stdout.
    pub fn resolve(
        &self,
        py_version: &PythonVersion,
        requirement: &Requirement,
        allow_prerelease: bool,
        exclude_newer: Option<String>,
        keyring_provider: KeyringProvider,
    ) -> Result<Requirement, Error> {
        let mut cmd = self.venv_cmd();
        let options = UvCompileOptions {
            allow_prerelease,
            exclude_newer,
            upgrade: UvPackageUpgrade::Nothing,
            no_deps: true,
            no_header: true,
            generate_hashes: false,
            keyring_provider,
            universal: false,
        };

        cmd.arg("pip").arg("compile");

        self.uv.sources.add_as_pip_args(&mut cmd);
        options.add_as_pip_args(&mut cmd);

        cmd.arg("--python-version").arg(py_version.format_simple());

        // We are using stdin so we can create the requirements in memory and don't
        // have to create a temporary file.
        cmd.arg("-");

        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Write requirement to stdin
        let child_stdin = child.stdin.as_mut().unwrap();
        writeln!(child_stdin, "{}", requirement)?;

        let rv = child.wait_with_output()?;
        if !rv.status.success() {
            let log = String::from_utf8_lossy(&rv.stderr);
            return Err(anyhow!(
                "Failed to run uv compile {}. uv exited with status: {}",
                log,
                rv.status
            ));
        }

        String::from_utf8_lossy(&rv.stdout)
            .parse()
            .context("unable to parse requirement from uv.")
    }
}
