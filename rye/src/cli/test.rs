use std::env::consts::EXE_EXTENSION;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Error};
use clap::Parser;
use console::style;
use same_file::is_same_file;

use crate::config::Config;
use crate::consts::VENV_BIN;
use crate::pyproject::{locate_projects, normalize_package_name, DependencyKind, PyProject};
use crate::sync::autosync;
use crate::utils::{CommandOutput, QuietExit};

/// Run the tests on the project.
///
/// Today this will always run `pytest` for all projects.
#[derive(Parser, Debug)]
pub struct Args {
    /// Perform the operation on all packages
    #[arg(short, long)]
    all: bool,
    /// Perform the operation on a specific package
    #[arg(short, long)]
    package: Vec<String>,
    /// Use this pyproject.toml file
    #[arg(long, value_name = "PYPROJECT_TOML")]
    pyproject: Option<PathBuf>,
    // Disable test output capture to stdout
    #[arg(long = "no-capture", short = 's')]
    no_capture: bool,
    /// Enables verbose diagnostics.
    #[arg(short, long)]
    verbose: bool,
    /// Turns off all output.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
    /// Extra arguments to pytest
    #[arg(last = true)]
    extra_args: Vec<OsString>,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);
    let project = PyProject::load_or_discover(cmd.pyproject.as_deref())?;

    let mut failed_with = None;

    // when working with workspaces we always want to know what other projects exist.
    // for that we locate all those projects and their paths.  This is later used to
    // prevent accidentally recursing into the wrong projects.
    let project_roots = if let Some(workspace) = project.workspace() {
        workspace
            .iter_projects()
            .filter_map(|x| x.ok())
            .map(|x| x.root_path().to_path_buf())
            .collect()
    } else {
        vec![project.root_path().to_path_buf()]
    };

    let pytest = project
        .venv_path()
        .join(VENV_BIN)
        .join("pytest")
        .with_extension(EXE_EXTENSION);

    let projects = locate_projects(project, cmd.all, &cmd.package[..])?;

    if !pytest.is_file() {
        let has_pytest = has_pytest_dependency(&projects)?;
        if has_pytest {
            if Config::current().autosync() {
                autosync(&projects[0], output)?;
            } else {
                bail!("pytest not installed but in dependencies. Run `rye sync`.")
            }
        } else {
            bail!("pytest not installed. Run `rye add --dev pytest`");
        }
    }

    for (idx, project) in projects.iter().enumerate() {
        if output != CommandOutput::Quiet {
            if idx > 0 {
                echo!();
            }
            echo!(
                "Running tests for {} ({})",
                style(project.name().unwrap_or("<unknown>")).cyan(),
                style(project.root_path().display()).dim()
            );
        }

        let mut pytest_cmd = Command::new(&pytest);
        if cmd.no_capture {
            pytest_cmd.arg("--capture=no");
        }
        match output {
            CommandOutput::Normal => {}
            CommandOutput::Verbose => {
                pytest_cmd.arg("-v");
            }
            CommandOutput::Quiet => {
                pytest_cmd.arg("-q");
            }
        }
        pytest_cmd.args(&cmd.extra_args);
        pytest_cmd
            .arg("--rootdir")
            .arg(project.root_path().as_os_str())
            .current_dir(project.root_path());

        // always ignore projects that are nested but not selected.
        for path in &project_roots {
            if !is_same_file(path, project.root_path()).unwrap_or(false) {
                pytest_cmd.arg("--ignore").arg(path.as_os_str());
            }
        }

        let status = pytest_cmd.status()?;
        if !status.success() {
            failed_with = Some(status.code().unwrap_or(1));
        }
    }

    if let Some(code) = failed_with {
        Err(Error::new(QuietExit(code)))
    } else {
        Ok(())
    }
}

/// Does any of those projects have a pytest dependency?
fn has_pytest_dependency(projects: &[PyProject]) -> Result<bool, Error> {
    for project in projects {
        for dep in project
            .iter_dependencies(DependencyKind::Dev)
            .chain(project.iter_dependencies(DependencyKind::Normal))
        {
            if let Ok(req) = dep.expand(|name| std::env::var(name).ok()) {
                if normalize_package_name(&req.name) == "pytest" {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}
