use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{anyhow, bail, Context, Error};
use clap::Parser;
use console::style;

use crate::bootstrap::{fetch, FetchOptions};
use crate::config::Config;

use crate::platform::get_toolchain_python_bin;
use crate::pyproject::{locate_projects, PyProject};
use crate::utils::{get_venv_python_bin, prepend_path_to_path_env, CommandOutput, IoPathContext};
use crate::uv::UvBuilder;

/// Builds a package for distribution.
#[derive(Parser, Debug)]
pub struct Args {
    /// Build a sdist
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
    let project = PyProject::load_or_discover(cmd.pyproject.as_deref())?;
    let py_ver = project.venv_python_version()?;

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

    let use_uv = Config::current().use_uv();
    let projects = locate_projects(project, cmd.all, &cmd.package[..])?;

    let all_virtual = projects.iter().all(|p| p.is_virtual());
    if all_virtual {
        warn!("skipping build, all projects are virtual");
        return Ok(());
    }

    // Make sure we have a compatible Python version.
    let py_ver = fetch(&py_ver.into(), FetchOptions::with_output(output))
        .context("failed fetching toolchain ahead of sync")?;
    echo!(if output, "Python version: {}", style(&py_ver).cyan());
    let py_bin = get_toolchain_python_bin(&py_ver)?;

    // Create a virtual environment in which to perform the builds.
    let uv = UvBuilder::new()
        .with_output(CommandOutput::Quiet)
        .ensure_exists()?;
    let venv_dir = tempfile::tempdir().context("failed to create temporary directory")?;
    let uv_venv = uv
        .venv(venv_dir.path(), &py_bin, &py_ver, None)
        .context("failed to create build environment")?;
    uv_venv.write_marker()?;
    uv_venv.bootstrap()?;

    // Respect the output level for the actual builds.
    let uv = uv.with_output(output);

    for project in projects {
        // skip over virtual packages on build
        if project.is_virtual() {
            continue;
        }

        echo!(
            if output,
            "building {}",
            style(project.normalized_name()?).cyan()
        );

        let mut build_cmd = Command::new(get_venv_python_bin(venv_dir.path()));
        build_cmd
            .arg("-mbuild")
            .env("NO_COLOR", "1")
            .arg("--outdir")
            .arg(&out)
            .arg(&*project.root_path());

        if use_uv {
            let uv_dir = uv
                .uv_bin()
                .parent()
                .ok_or_else(|| anyhow!("Could not find uv binary in self venv: empty path"))?;
            build_cmd.env("PATH", prepend_path_to_path_env(uv_dir)?);
            build_cmd.arg("--installer=uv");
        }

        if cmd.wheel {
            build_cmd.arg("--wheel");
        }
        if cmd.sdist {
            build_cmd.arg("--sdist");
        }

        if output == CommandOutput::Verbose {
            build_cmd.arg("--verbose");
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
