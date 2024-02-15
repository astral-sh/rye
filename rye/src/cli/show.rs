use std::path::Path;
use std::path::PathBuf;

use anyhow::Error;
use clap::Parser;
use console::style;

use crate::pyproject::{get_current_venv_python_version, PyProject};

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
    if cmd.installed_deps {
        warn!("--installed-deps is deprecated, use `rye list`");
        return crate::cli::list::execute(crate::cli::list::Args {
            pyproject: cmd.pyproject,
        });
    }

    let project = PyProject::load_or_discover(cmd.pyproject.as_deref())?;
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
    echo!("virtual: {}", style(project.is_virtual()).cyan());

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

    match project.sources() {
        Ok(mut sources) => {
            sources.sort_by_cached_key(|x| (x.name != "default", x.name.to_string()));
            echo!("configured sources:");
            for source in sources {
                echo!(
                    "  {} ({}: {})",
                    style(&source.name).cyan(),
                    style(&source.ty).yellow(),
                    style(&source.url).dim(),
                );
            }
        }
        Err(err) => echo!("invalid source config: {}", style(err).red()),
    }

    Ok(())
}
