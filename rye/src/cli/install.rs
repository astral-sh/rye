use std::path::Path;

use anyhow::{Context, Error};
use clap::Parser;
use pep508_rs::Requirement;

use crate::cli::add::ReqExtras;
use crate::installer::{install, resolve_local_requirement};
use crate::sources::PythonVersionRequest;
use crate::utils::CommandOutput;

/// Installs a package as global tool.
#[derive(Parser, Debug)]
pub struct Args {
    /// The name of the package to install.
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

pub fn execute(mut cmd: Args) -> Result<(), Error> {
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);

    let mut requirement = match resolve_local_requirement(Path::new(&cmd.requirement), output)? {
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

    install(requirement, &py_ver, cmd.force, &cmd.include_dep, output)?;
    Ok(())
}
