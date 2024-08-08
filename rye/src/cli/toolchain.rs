use std::cmp::Reverse;
use std::collections::HashMap;
use std::env::consts::{ARCH, OS};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Error};
use clap::Parser;
use clap::ValueEnum;
use console::style;
use serde::Deserialize;
use serde::Serialize;

use crate::installer::list_installed_tools;
use crate::piptools::get_pip_tools_venv_path;
use crate::platform::{get_app_dir, get_canonical_py_path, list_known_toolchains};
use crate::pyproject::read_venv_marker;
use crate::sources::py::{iter_downloadable, PythonVersion};
use crate::utils::{symlink_file, IoPathContext};

const INSPECT_SCRIPT: &str = r#"
import json
import platform
import sysconfig
print(json.dumps({
    "python_implementation": platform.python_implementation(),
    "python_version": platform.python_version(),
    "python_debug": bool(sysconfig.get_config_var('Py_DEBUG')),
}))
"#;

#[derive(Debug, Deserialize)]
struct InspectInfo {
    python_implementation: String,
    python_version: String,
    python_debug: bool,
}

/// Helper utility to manage Python toolchains.
#[derive(Parser, Debug)]
pub struct Args {
    #[command(subcommand)]
    command: SubCommand,
}

/// Register a Python binary.
///
/// Rye by default will automatically download Python releases from the internet.
/// However it's also possible to register already available local Python
/// installations.  This allows you to use rye with self compiled Pythons.
#[derive(Parser, Debug)]
pub struct RegisterCommand {
    /// Path to the Python binary.
    path: PathBuf,
    /// Name of the toolchain.  If not provided a name is auto detected.
    #[arg(short, long)]
    name: Option<String>,
}

/// Removes a toolchain.
#[derive(Parser, Debug)]
pub struct RemoveCommand {
    /// Name and version of the toolchain.
    version: String,
    /// Force removal even if the toolchain is in use.
    #[arg(short, long)]
    force: bool,
}

/// List all registered toolchains
#[derive(Parser, Debug)]
pub struct ListCommand {
    /// Also include non installed, but downloadable toolchains
    #[arg(long)]
    include_downloadable: bool,
    /// Request parseable output format
    #[arg(long)]
    format: Option<Format>,
}

#[derive(ValueEnum, Copy, Clone, Serialize, Debug, PartialEq)]
#[value(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
enum Format {
    Json,
}

#[derive(Parser, Debug)]
enum SubCommand {
    Fetch(crate::cli::fetch::Args),
    List(ListCommand),
    Register(RegisterCommand),
    Remove(RemoveCommand),
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    match cmd.command {
        SubCommand::Register(args) => register(args),
        SubCommand::Fetch(args) => crate::cli::fetch::execute(args),
        SubCommand::List(args) => list(args),
        SubCommand::Remove(args) => remove(args),
    }
}

fn register(cmd: RegisterCommand) -> Result<(), Error> {
    let mut toolchain_builder = ToolchainBuilder::new(&cmd.path, cmd.name.as_deref());
    toolchain_builder.find()?;
    toolchain_builder.validate(|_| Ok(()))?;
    let toolchain = toolchain_builder.register()?;
    echo!("Registered {} as {}", cmd.path.display(), toolchain);
    Ok(())
}

/// Checks if a toolchain is still in use.
fn check_in_use(ver: &PythonVersion) -> Result<(), Error> {
    // Check if used by rye itself.
    let app_dir = get_app_dir();
    for venv in &[app_dir.join("self"), get_pip_tools_venv_path(ver)] {
        let venv_marker = read_venv_marker(venv);
        if let Some(ref venv_marker) = venv_marker {
            if &venv_marker.python == ver {
                bail!("toolchain {} is still in use by rye itself", ver);
            }
        }
    }

    // Check if used by any tool.
    let installed_tools = list_installed_tools()?;
    for (tool, info) in &installed_tools {
        if let Some(ref venv_marker) = info.venv_marker {
            if &venv_marker.python == ver {
                bail!("toolchain {} is still in use by tool {}", ver, tool);
            }
        }
    }

    Ok(())
}

pub fn remove(cmd: RemoveCommand) -> Result<(), Error> {
    let ver: PythonVersion = cmd.version.parse()?;
    let path = get_canonical_py_path(&ver)?;

    if !cmd.force && path.exists() {
        check_in_use(&ver)?;
    }

    if path.is_file() {
        fs::remove_file(&path).path_context(&path, "failed to remove toolchain link")?;
        echo!("Removed toolchain link {}", &ver);
    } else if path.is_dir() {
        fs::remove_dir_all(&path).path_context(&path, "failed to remove toolchain")?;
        echo!("Removed installed toolchain {}", &ver);
    } else {
        echo!("Toolchain is not installed");
    }
    Ok(())
}

/// Output structure for toolchain list --format=json
// Reserves the right to expand with new fields.
#[derive(Serialize)]
struct ListVersion {
    name: PythonVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    downloadable: Option<bool>,
}

fn secondary_architectures() -> &'static [&'static str] {
    match (OS, ARCH) {
        ("windows", "x86_64") => &["x86"],
        ("windows", "aarch64") => &["x86_64", "x86"],
        ("macos", "aarch64") => &["x86_64"],
        _ => &[],
    }
}

fn list(cmd: ListCommand) -> Result<(), Error> {
    let mut toolchains = list_known_toolchains()?
        .into_iter()
        .map(|(version, path)| (version, Some(path)))
        .collect::<HashMap<_, _>>();

    if cmd.include_downloadable {
        for version in iter_downloadable(OS, ARCH) {
            toolchains.entry(version).or_insert(None);
        }
        for secondary_arch in secondary_architectures() {
            for version in iter_downloadable(OS, secondary_arch) {
                toolchains.entry(version).or_insert(None);
            }
        }
    }

    let mut versions = toolchains.into_iter().collect::<Vec<_>>();
    versions.sort_by_cached_key(|a| (a.1.is_none(), a.0.name.to_string(), Reverse(a.clone())));

    if let Some(Format::Json) = cmd.format {
        let json_versions = versions
            .into_iter()
            .map(|(version, path)| ListVersion {
                name: version,
                downloadable: if path.is_none() { Some(true) } else { None },
                path: path.map(|p| p.to_string_lossy().into_owned()),
            })
            .collect::<Vec<_>>();
        serde_json::to_writer_pretty(std::io::stdout().lock(), &json_versions)?;
        echo!();
    } else {
        for (version, path) in versions {
            if let Some(path) = path {
                echo!(
                    "{} ({})",
                    style(&version).green(),
                    style(path.display()).dim()
                );
            } else {
                echo!("{} (downloadable)", style(version).dim());
            }
        }
    }
    Ok(())
}

/// Find, validate, and register a custom Python toolchain
#[derive(Debug)]
pub struct ToolchainBuilder {
    path: PathBuf,
    name: Option<String>,
    requested_target_version: Option<PythonVersion>,
    validated_target_version: Option<PythonVersion>,
}

impl ToolchainBuilder {
    pub fn new(path: &Path, name: Option<&str>) -> Self {
        ToolchainBuilder {
            path: path.to_owned(),
            name: name.map(String::from),
            requested_target_version: None,
            validated_target_version: None,
        }
    }

    pub fn find(&mut self) -> anyhow::Result<&Self, Error> {
        let output = Command::new(&self.path)
            .arg("-c")
            .arg(INSPECT_SCRIPT)
            .output()
            .context("error executing interpreter to inspect version")?;

        if !output.status.success() {
            bail!("passed path does not appear to be a valid Python installation");
        }

        let info: InspectInfo = serde_json::from_slice(&output.stdout)
            .context("could not parse interpreter output as json")?;

        let target_version = self
            .parse_target_version(info)
            .context("could not parse target version")?;

        self.requested_target_version = Some(target_version);

        Ok(self)
    }

    fn parse_target_version(&self, info: InspectInfo) -> anyhow::Result<PythonVersion, Error> {
        let target_version_str = match &self.name {
            Some(ref name) => format!("{}@{}", name, info.python_version),
            None => {
                format!(
                    "{}{}@{}",
                    info.python_implementation.to_ascii_lowercase(),
                    if info.python_debug { "-dbg" } else { "" },
                    info.python_version
                )
            }
        };

        let target_version: PythonVersion = target_version_str.parse()?;

        Ok(target_version)
    }

    pub fn validate<F>(&mut self, validate: F) -> anyhow::Result<&Self, Error>
    where
        F: FnOnce(&PythonVersion) -> Result<(), Error>,
    {
        let requested_target_version = self
            .requested_target_version
            .as_ref()
            .ok_or_else(|| anyhow!("toolchain has not been inquired yet"))?;

        validate(requested_target_version)
            .with_context(|| format!("{} is not a valid toolchain", requested_target_version))?;

        self.validated_target_version = Some(requested_target_version.clone());

        Ok(self)
    }

    pub fn register(&self) -> anyhow::Result<PythonVersion, Error> {
        let validated_target_version = self
            .validated_target_version
            .as_ref()
            .ok_or_else(|| anyhow!("toolchain has not been validated yet"))?;

        let canonical_target_path = get_canonical_py_path(validated_target_version)?;

        // Prepare the canonical target path
        self.prepare_target_path(&canonical_target_path)?;

        #[cfg(unix)]
        self.symlink_or_fallback::<Unix>(&self.path, &canonical_target_path)?;

        #[cfg(windows)]
        self.symlink_or_fallback::<Windows>(&self.path, &canonical_target_path)?;

        Ok(validated_target_version.to_owned())
    }

    fn prepare_target_path(&self, target: &Path) -> anyhow::Result<()> {
        // Check if the target path exists to avoid overwriting
        if target.is_file() || target.is_dir() {
            bail!("Target Python path {} is already in use", target.display());
        }

        // Ensure the parent directory exists
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Could not create directory {}", parent.display()))?;
        }

        Ok(())
    }

    fn symlink_or_fallback<S: SymlinkOrFallback>(
        &self,
        src: &Path,
        dst: &Path,
    ) -> anyhow::Result<()> {
        S::symlink_or_fallback(src, dst)
    }
}

trait SymlinkOrFallback {
    fn symlink_or_fallback(src: &Path, dst: &Path) -> anyhow::Result<()>;
}

#[cfg(unix)]
struct Unix;

#[cfg(unix)]
impl SymlinkOrFallback for Unix {
    /// On Unix-like systems, creating symlinks does not require elevated privileges and is a common operation.
    /// This function attempts to create a symlink and does not need a fallback mechanism,
    /// as the permission model typically allows all users to create symlinks.
    fn symlink_or_fallback(src: &Path, dst: &Path) -> anyhow::Result<()> {
        symlink_file(src, dst).context("could not symlink interpreter")
    }
}

#[cfg(windows)]
struct Windows;

#[cfg(windows)]
impl SymlinkOrFallback for Windows {
    /// On Windows, creating symlinks requires elevated privileges not commonly granted to all users.
    /// If symlink creation fails, this function falls back to writing the source path to the destination file.
    /// This ensures functionality across various permission levels, maintaining cross-platform compatibility.
    fn symlink_or_fallback(src: &Path, dst: &Path) -> anyhow::Result<()> {
        symlink_file(src, dst).or_else(|_| {
            let src_str = src
                .as_os_str()
                .to_str()
                .ok_or_else(|| anyhow!("non-unicode path to interpreter"))?;
            fs::write(dst, src_str).context("could not register interpreter")
        })?;

        Ok(())
    }
}
