use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{bail, Error};
use clap::Parser;
use console::style;

use crate::bootstrap::ensure_self_venv;
use crate::pyproject::{locate_projects, PyProject};
use crate::utils::{get_venv_python_bin, CommandOutput, IoPathContext};

/// Builds a package for distribution.
#[derive(Parser, Debug)]
pub struct Args {
    /// Build an sdist
    #[arg(long)]
    sdist: bool,
    /// Build a wheel
    #[arg(long)]
    wheel: bool,
    /// Build all packages
    #[arg(short, long)]
    all: bool,
    /// Build a specific package
    #[arg(short, long)]
    package: Vec<String>,
    /// An output directory (defaults to `workspace/dist`)
    #[arg(short, long)]
    out: Option<PathBuf>,
    /// Use this pyproject.toml file
    #[arg(long, value_name = "PYPROJECT_TOML")]
    pyproject: Option<PathBuf>,
    /// Clean the output directory first
    #[arg(short, long)]
    clean: bool,
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
    let project = PyProject::load_or_discover(cmd.pyproject.as_deref())?;

    let out = match cmd.out {
        Some(path) => path,
        None => project.workspace_path().join("dist"),
    };

    if out.exists() && cmd.clean {
        for entry in fs::read_dir(&out).path_context(&out, "enumerate build output")? {
            let path = entry?.path();
            if path.is_file() {
                fs::remove_file(&path).path_context(&path, "clean build artifact")?;
            }
        }
    }

    let projects = locate_projects(project, cmd.all, &cmd.package[..])?;

    for project in projects {
        // skip over virtual packages on build
        if project.is_virtual() {
            continue;
        }

        if output != CommandOutput::Quiet {
            echo!("building {}", style(project.normalized_name()?).cyan());
        }

        let mut build_cmd = Command::new(get_venv_python_bin(&venv));
        build_cmd
            .arg("-mbuild")
            .env("NO_COLOR", "1")
            .arg("--outdir")
            .arg(&out)
            .arg(&*project.root_path());

        if cmd.wheel {
            build_cmd.arg("--wheel");
        }
        if cmd.sdist {
            build_cmd.arg("--sdist");
        }

        if output == CommandOutput::Quiet {
            build_cmd.stdout(Stdio::null());
            build_cmd.stderr(Stdio::null());
        }

        let status = build_cmd.status()?;
        if !status.success() {
            bail!("failed to build dist");
        }
    }

    Ok(())
}
