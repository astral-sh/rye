use std::path::Path;

use anyhow::{Context, Error};
use clap::Parser;
use pep508_rs::Requirement;

use crate::cli::add::ReqExtras;
use crate::config::Config;
use crate::installer::{install, resolve_local_requirement};
use crate::sources::py::PythonVersionRequest;
use crate::utils::CommandOutput;

/// Installs a package as global tool. This is an alias of `rye tools install`.
#[derive(Parser, Debug)]
pub struct Args {
    /// The name of the package to install.
    requirement: String,
    #[command(flatten)]
    req_extras: ReqExtras,
    /// Include scripts from a given dependency.
    #[arg(long)]
    include_dep: Vec<String>,
    /// Additional dependencies to install that are not declared by the main package.
    #[arg(long)]
    extra_requirement: Vec<String>,
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
    let mut extra_requirements = Vec::new();

    // main requirement
    let mut requirement = handle_requirement(&cmd.requirement, output, true)?;
    // installations here always use absolute paths for local references
    // because we do not have a rye workspace to work with.
    cmd.req_extras.force_absolute();
    cmd.req_extras.apply_to_requirement(&mut requirement)?;

    for req in cmd.extra_requirement {
        extra_requirements.push(handle_requirement(&req, output, false)?);
    }

    let py_ver: PythonVersionRequest = match cmd.python {
        Some(ref py) => py.parse()?,
        None => Config::current()
            .default_toolchain()
            .unwrap_or(PythonVersionRequest {
                name: None,
                arch: None,
                os: None,
                environment: None,
                major: 3,
                minor: None,
                patch: None,
                suffix: None,
            }),
    };

    install(
        requirement,
        &py_ver,
        cmd.force,
        &cmd.include_dep,
        &extra_requirements,
        output,
    )?;
    Ok(())
}

fn handle_requirement(
    req: &str,
    output: CommandOutput,
    local_hint: bool,
) -> Result<Requirement, Error> {
    Ok(match resolve_local_requirement(Path::new(req), output)? {
        Some(req) => req,
        None => req.parse::<Requirement>().with_context(|| {
            if local_hint && req.contains("://") {
                format!(
                    "failed to parse requirement '{}'. It looks like a URL, maybe \
                        you wanted to use --url or --git",
                    req
                )
            } else {
                format!("failed to parse requirement '{}'", req)
            }
        })?,
    })
}
