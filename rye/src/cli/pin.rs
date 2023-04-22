use std::env::{
    self,
    consts::{ARCH, OS},
};
use std::fs;

use anyhow::{anyhow, Error};
use clap::Parser;

use crate::{pyproject::PyProject, sources::get_download_url};

/// Pins a Python version to this project.
#[derive(Parser, Debug)]
pub struct Args {
    /// The version of Python to fetch.
    version: String,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let (version, _) = get_download_url(&cmd.version, OS, ARCH)
        .ok_or_else(|| anyhow!("unsupported version for this platform"))?;

    // pin in a format known to other toolchains for as long as we're under cpython
    let serialized_version = version.to_string();
    let to_write = if let Some(rest) = serialized_version.strip_prefix("cpython@") {
        rest
    } else {
        &serialized_version
    };

    let version_file = match PyProject::discover() {
        Ok(proj) => proj.root_path().join(".python-version"),
        Err(_) => env::current_dir()?.join(".python-version"),
    };
    fs::write(&version_file, format!("{}\n", to_write))?;

    eprintln!("pinned {} in {}", version, version_file.display());

    Ok(())
}
