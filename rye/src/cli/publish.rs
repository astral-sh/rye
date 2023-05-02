use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{bail, Error};
use clap::Parser;

use crate::bootstrap::ensure_self_venv;
use crate::pyproject::PyProject;
use crate::utils::CommandOutput;

/// Builds a package for distribution.
#[derive(Parser, Debug)]
pub struct Args {
    /// The distribution files to upload to the repository (defaults to <workspace-root>/dist/*).
    dist: Option<Vec<PathBuf>>,
    /// The repository url to publish to (default is https://upload.pypi.org/legacy/).
    #[arg(long, default_value = "https://upload.pypi.org/legacy/")]
    repository_url: String,
    /// Sign files to upload using GPG.
    #[arg(long)]
    sign: bool,
    /// GPG identity used to sign files.
    #[arg(short, long)]
    identity: Option<String>,
    /// Path to alternate CA bundle.
    #[arg(long)]
    cert: Option<PathBuf>,
    /// Path to the .pypirc config file to use.
    #[arg(long)]
    config_file: Option<PathBuf>,
    /// Enables verbose diagnostics.
    #[arg(short, long)]
    verbose: bool,
    /// Turns off all output.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);
    let venv = ensure_self_venv(output)?;
    let project = PyProject::discover()?;

    let files = match cmd.dist {
        Some(paths) => paths,
        None => vec![project.workspace_path().join("dist").join("*")],
    };

    let mut publish_cmd = Command::new(venv.join("bin/python"));
    publish_cmd
        .arg("-mtwine")
        .arg("--no-color")
        .arg("upload")
        .args(files)
        .arg("--repository-url")
        .arg(cmd.repository_url);
    if cmd.sign {
        publish_cmd.arg("--sign");
    }
    if let Some(identity) = cmd.identity {
        publish_cmd.arg("--identity").arg(identity);
    }
    if let Some(config_path) = cmd.config_file {
        publish_cmd.arg("--config-file").arg(config_path);
    }
    if let Some(cert) = cmd.cert {
        publish_cmd.arg("--cert").arg(cert);
    }

    if output == CommandOutput::Quiet {
        publish_cmd.stdout(Stdio::null());
        publish_cmd.stderr(Stdio::null());
    }

    let status = publish_cmd.status()?;
    if !status.success() {
        bail!("failed to build dist");
    }

    Ok(())
}
