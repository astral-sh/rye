use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Error};
use clap::Parser;
use console::style;

use crate::bootstrap::ensure_self_venv;
use crate::consts::VENV_BIN;
use crate::pyproject::{get_current_venv_python_version, PyProject};
use crate::utils::{get_venv_python_bin, CommandOutput};

/// Prints the current state of the project.
#[derive(Parser, Debug)]
pub struct Args {
    /// Print the installed dependencies from the venv
    #[arg(long)]
    installed_deps: bool,
    /// Use this pyproject.toml file
    #[arg(long, value_name = "PYPROJECT_TOML")]
    pyproject: Option<PathBuf>,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let project = PyProject::load_or_discover(cmd.pyproject.as_deref())?;

    if cmd.installed_deps {
        return print_installed_deps(&project);
    }

    echo!(
        "project: {}",
        style(project.name().unwrap_or("<unnamed>")).yellow()
    );
    echo!("path: {}", style(project.root_path().display()).cyan());
    echo!("venv: {}", style(project.venv_path().display()).cyan());
    if let Some(ver) = project.target_python_version() {
        echo!("target python: {}", style(ver).cyan());
    }
    if let Ok(ver) = project.venv_python_version() {
        echo!("venv python: {}", style(&ver).cyan());
        if let Some(actual) = get_current_venv_python_version(&project.venv_path()) {
            if actual != ver {
                echo!("last synched venv python: {}", style(&actual).red());
            }
        }
    }

    if let Some(workspace) = project.workspace() {
        echo!(
            "workspace: {}",
            style(project.workspace_path().display()).cyan()
        );
        echo!("  members:");
        let mut projects = workspace.iter_projects().collect::<Result<Vec<_>, _>>()?;
        projects.sort_by(|a, b| a.root_path().cmp(&b.root_path()));
        for child in projects {
            let root_path = child.root_path();
            let rel_path = Path::new(".").join(
                root_path
                    .strip_prefix(project.workspace_path())
                    .unwrap_or(&root_path),
            );
            echo!(
                "    {} ({})",
                style(child.name().unwrap_or("<unnamed>")).cyan(),
                style(rel_path.display()).dim(),
            );
        }
    }

    Ok(())
}

fn print_installed_deps(project: &PyProject) -> Result<(), Error> {
    let python = get_venv_python_bin(&project.venv_path());
    if !python.is_file() {
        return Ok(());
    }
    let self_venv = ensure_self_venv(CommandOutput::Normal)?;

    let status = Command::new(self_venv.join(VENV_BIN).join("pip"))
        .arg("--python")
        .arg(&python)
        .arg("freeze")
        .env("PYTHONWARNINGS", "ignore")
        .env("PIP_DISABLE_PIP_VERSION_CHECK", "1")
        .status()?;

    if !status.success() {
        bail!("failed to print dependencies via pip");
    }

    Ok(())
}
