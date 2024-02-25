use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use anyhow::Error;
use clap::Parser;
use pep508_rs::{CharIter, Requirement};

use crate::consts::VENV_BIN;
use crate::pyproject::DependencyKind;
use crate::pyproject::{locate_projects, PyProject};
use crate::utils::{CommandOutput, QuietExit};

#[derive(Parser, Debug)]
pub struct PyTestArgs {
    /// Perform the operation on all packages
    #[arg(short, long)]
    all: bool,
    /// Perform the operation on a specific package
    #[arg(short, long)]
    package: Vec<String>,
    /// Use this pyproject.toml file
    #[arg(long, value_name = "PYPROJECT_TOML")]
    pyproject: Option<PathBuf>,
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

const PYTEST_DEPENDENCY: &str = "pytest==8.0.2";

pub fn execute_pytest(args: PyTestArgs, extra_args: &[String]) -> Result<(), Error> {
    let project = PyProject::load_or_discover(args.pyproject.as_deref())?;
    let output = CommandOutput::from_quiet_and_verbose(args.quiet, args.verbose);
    // find pytest from project because pytest needs to access local packages
    let pytest = project.venv_path().join(VENV_BIN).join("pytest");

    let mut pytest_cmd = Command::new(pytest);

    match output {
        CommandOutput::Normal => {}
        CommandOutput::Verbose => {
            pytest_cmd.arg("--verbose");
        }
        CommandOutput::Quiet => {
            pytest_cmd.arg("-q");
        }
    }
    pytest_cmd.args(extra_args);
    pytest_cmd.args(args.extra_args);

    pytest_cmd.arg("--");

    // if the package is managed by rye, we can update dev-dependencies and sync the project
    let mut need_sync = false;

    let projects = locate_projects(project, args.all, &args.package[..])?;
    for mut project in projects {
        // is_workspace_root returns true if the workspace is missing...
        if project.workspace().is_some() && project.is_workspace_root() {
            // don't install pytest in the root workspace as each package should be self consistent
            continue;
        }
        // search for pytests in [dev-dependencies]
        let installed_pytest = project.search_dependency_by_name("pytest", DependencyKind::Dev);

        if installed_pytest.is_none() && !project.rye_managed() {
            // pytest is missing and rye doesn't manage the workspace, we have no consent to update the pyproject.toml, error out and notify
            return Err(anyhow::anyhow!("Unmanaged rye project, pytest should be part of [dev-dependencies] in order to use `rye test`"));
        }

        if installed_pytest.is_none() {
            // pytest is missing but rye manages the package, alert the user and the workspace needs to synced
            warn!("This project is managed by rye, {PYTEST_DEPENDENCY} will be added to the [dev-dependencies] of {} in order to use `rye test`, workspace will be synced", project.name().unwrap_or(""));
            project.add_dependency(
                &Requirement::parse(&mut CharIter::new(PYTEST_DEPENDENCY))?,
                &DependencyKind::Dev,
            )?;
            project.save()?;
            need_sync = true;
        }

        pytest_cmd.arg(project.root_path().as_os_str());
    }

    if need_sync {
        // only sync pytest
        crate::cli::sync::execute(crate::cli::sync::Args::parse_from(["--update=pytest"]))?;
    }

    let status = pytest_cmd.status()?;
    if !status.success() {
        let code = status.code().unwrap_or(1);
        Err(QuietExit(code).into())
    } else {
        Ok(())
    }
}
