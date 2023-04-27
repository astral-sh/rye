use anyhow::{bail, Context, Error};
use clap::Parser;
use pep508_rs::Requirement;

use crate::installer::install;
use crate::sources::PythonVersionRequest;
use crate::utils::CommandOutput;

/// Installs a package as global tool.
#[derive(Parser, Debug)]
pub struct Args {
    /// The name of the package to install.
    requirement: String,
    /// Install the given package from this git repository
    #[arg(long)]
    git: Option<String>,
    /// Install the given package from this URL
    #[arg(long, conflicts_with = "git")]
    url: Option<String>,
    /// Install a specific tag.
    #[arg(long, requires = "git")]
    tag: Option<String>,
    /// Update to a specific git rev.
    #[arg(
        long,
        conflicts_with = "tag",
        conflicts_with = "branch",
        requires = "git"
    )]
    rev: Option<String>,
    /// Update to a specific git branch.
    #[arg(long, conflicts_with = "tag", conflicts_with = "rev", requires = "git")]
    branch: Option<String>,
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

pub fn execute(cmd: Args) -> Result<(), Error> {
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);

    let mut requirement: Requirement = cmd
        .requirement
        .parse()
        .with_context(|| {
            if cmd.requirement.contains("://") {
                format!("failed to parse requirement '{}'. It looks like a URL, maybe you wanted to use --url or --git", cmd.requirement)
            } else {
                format!("failed to parse requirement '{}'", cmd.requirement)
            }
        })?;
    if let Some(ref git) = cmd.git {
        // XXX: today they are all aliases, it might be better to change
        // tag to refs/tags/<tag> and branch to refs/heads/<branch> but
        // this creates some ugly warnings in pip today
        let suffix = match cmd
            .rev
            .as_ref()
            .or(cmd.tag.as_ref())
            .or(cmd.branch.as_ref())
        {
            Some(rev) => format!("@{}", rev),
            None => "".into(),
        };
        requirement.version_or_url = match requirement.version_or_url {
            Some(_) => bail!("requirement already has a version marker"),
            None => Some(pep508_rs::VersionOrUrl::Url(
                format!("git+{}{}", git, suffix).parse().with_context(|| {
                    format!("unable to interpret '{}{}' as git reference", git, suffix)
                })?,
            )),
        };
    } else if let Some(ref url) = cmd.url {
        requirement.version_or_url = match requirement.version_or_url {
            Some(_) => bail!("requirement already has a version marker"),
            None => Some(pep508_rs::VersionOrUrl::Url(
                url.parse()
                    .with_context(|| format!("unable to parse '{}' as url", url))?,
            )),
        };
    }

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

    install(requirement, &py_ver, cmd.force, output)?;
    Ok(())
}
