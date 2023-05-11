use std::cmp::Reverse;
use std::collections::HashMap;
use std::env::consts::{ARCH, OS};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Error};
use clap::Parser;
use console::style;
use serde::Deserialize;

use crate::platform::{get_canonical_py_path, list_known_toolchains};
use crate::sources::{iter_downloadable, PythonVersion};
use crate::utils::symlink_file;

const INSPECT_SCRIPT: &str = r#"
import json
import platform
print(json.dumps({
    "python_implementation": platform.python_implementation(),
    "python_version": platform.python_version(),
}))
"#;

#[derive(Debug, Deserialize)]
struct InspectInfo {
    python_implementation: String,
    python_version: String,
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
}

/// List all registered toolchains
#[derive(Parser, Debug)]
pub struct ListCommand {
    /// Also include non installed, but downloadable toolchains
    #[arg(long)]
    include_downloadable: bool,
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
    let output = Command::new(&cmd.path)
        .arg("-c")
        .arg(INSPECT_SCRIPT)
        .output()
        .context("error executing interpreter to inspect version")?;
    if !output.status.success() {
        bail!("passed path does not appear to be a valid Python installation");
    }

    let info: InspectInfo = serde_json::from_slice(&output.stdout)
        .context("could not parse interpreter output as json")?;
    let target_version = match cmd.name {
        Some(ref name) => format!("{}@{}", name, info.python_version),
        None => {
            let name = if info.python_implementation.eq_ignore_ascii_case("cpython") {
                "custom-cpython"
            } else {
                &info.python_implementation
            };
            format!("{}@{}", name.to_ascii_lowercase(), info.python_version)
        }
    };
    let target_version: PythonVersion = target_version.parse()?;
    let target = get_canonical_py_path(&target_version)?;

    if target.is_file() || target.is_dir() {
        bail!("target Python path {} is already in use", target.display());
    }

    // for the unlikely case that no python installation has been bootstrapped yet
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).ok();
    }

    // XXX: this requires elevated privileges on windows but using a hardlink here would
    // break the experience because then the interpreter does not know where it's from.
    // maybe we want to place files there containing the path to the interpreter instead.
    symlink_file(&cmd.path, target).context("could not symlink interpreter")?;
    println!("Registered {} as {}", cmd.path.display(), target_version);

    Ok(())
}

pub fn remove(cmd: RemoveCommand) -> Result<(), Error> {
    let ver: PythonVersion = cmd.version.parse()?;
    let path = get_canonical_py_path(&ver)?;
    if path.is_file() {
        fs::remove_file(&path)?;
        eprintln!("Removed toolchain link {}", &ver);
    } else if path.is_dir() {
        fs::remove_dir_all(&path)?;
        eprintln!("Removed installed toolchain {}", &ver);
    } else {
        eprintln!("Toolchain is not installed");
    }
    Ok(())
}

fn list(cmd: ListCommand) -> Result<(), Error> {
    let mut toolchains = list_known_toolchains()?
        .into_iter()
        .map(|version| (version, true))
        .collect::<HashMap<_, _>>();

    if cmd.include_downloadable {
        for version in iter_downloadable(OS, ARCH) {
            toolchains.entry(version).or_insert(false);
        }
    }

    let mut versions = toolchains.into_iter().collect::<Vec<_>>();
    versions.sort_by_cached_key(|a| (!a.1, a.0.kind.to_string(), Reverse(a.clone())));

    for (version, installed) in versions {
        if installed {
            println!("{}", style(&version).green());
        } else {
            println!("{} (downloadable)", style(version).dim());
        }
    }
    Ok(())
}
