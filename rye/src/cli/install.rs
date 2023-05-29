use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Error};
use clap::Parser;
use pep508_rs::Requirement;

use crate::cli::add::ReqExtras;
use crate::cli::uninstall::get_project_name_in_current_directory;
use crate::installer::{install, resolve_local_requirement, uninstall};
use crate::pyproject::{normalize_package_name, PyProject, Workspace};
use crate::sources::PythonVersionRequest;
use crate::utils::CommandOutput;

/// Installs a package as global tool.
#[derive(Parser, Debug)]
pub struct Args {
    /// The name of the package to install.
    #[arg(default_value = ".")]
    requirement: String,
    #[command(flatten)]
    req_extras: ReqExtras,
    /// Include scripts from a given dependency.
    #[arg(long)]
    include_dep: Vec<String>,
    /// Optionally the Python version to use.
    #[arg(short, long)]
    python: Option<String>,
    /// Force install the package even if it's already there.
    #[arg(short, long)]
    force: bool,
    /// Enables verbose diagnostics.
    #[arg(short, long)]
    verbose: bool,
    /// Turns off all output.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
}

fn resolve_requirement_to_path(req: &str, workspace: Arc<Workspace>) -> Result<PathBuf, Error> {
    let normalized_name = normalize_package_name(req);
    for project in workspace.iter_projects() {
        let project = project?;
        if project.normalized_name()? == normalized_name {
            return Ok(project.root_path().into());
        }
    }
    Ok(req.into())  // No match, return original string as path
}

pub fn execute(mut cmd: Args) -> Result<(), Error> {
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);

    let project = PyProject::discover()?;
    let workspace = project.workspace().unwrap(); // Get the workspace

    // Try to resolve the requirement to an absolute path if it matches a project
    // If the requirement is ".", use the absolute path of the current directory
    let requirement = if cmd.requirement == "." {
        uninstall(get_project_name_in_current_directory()?.as_str(), output.clone())?; // Uninstall the current project (if it exists
        env::current_dir()?.canonicalize()?
    } else {
        resolve_requirement_to_path(&cmd.requirement, workspace.clone())?
    };

    let mut requirement = match resolve_local_requirement(&requirement, output)? {
        Some(req) => req,
        None => cmd.requirement.parse::<Requirement>().with_context(|| {
            if cmd.requirement.contains("://") {
                format!(
                    "failed to parse requirement '{}'. It looks like a URL, maybe \
                        you wanted to use --url or --git",
                    cmd.requirement
                )
            } else {
                format!("failed to parse requirement '{}'", cmd.requirement)
            }
        })?,
    };

    // installations here always use absolute paths for local references
    // because we do not have a rye workspace to work with.
    cmd.req_extras.force_absolute();
    cmd.req_extras.apply_to_requirement(&mut requirement)?;

    let py_ver: PythonVersionRequest = match cmd.python {
        Some(ref py) => py.parse()?,
        None => PythonVersionRequest {
            kind: None,
            major: 3,
            minor: None,
            patch: None,
            suffix: None,
        },
    };

    env::set_current_dir(workspace.path()).expect("TODO: panic message");
    install(requirement, &py_ver, cmd.force, &cmd.include_dep, output)?;
    Ok(())
}
