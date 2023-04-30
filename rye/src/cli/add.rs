use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;

use anyhow::{anyhow, bail, Context, Error};
use clap::Parser;
use pep440_rs::VersionSpecifiers;
use pep508_rs::{Requirement, VersionOrUrl};
use serde::Deserialize;
use url::Url;

use crate::bootstrap::ensure_self_venv;
use crate::pyproject::{DependencyKind, PyProject};
use crate::utils::{format_requirement, CommandOutput};

const PACKAGE_FINDER_SCRIPT: &str = r#"
import sys
import json
from unearth.finder import PackageFinder
from packaging.version import Version

py_ver = tuple(map(int, sys.argv[1].split('.')))
package = sys.argv[2]
pre = len(sys.argv) > 3 and sys.argv[3] == "--pre"

finder = PackageFinder(
    index_urls=["https://pypi.org/simple/"],
)
finder.target_python.py_ver = py_ver
choices = iter(finder.find_matches(package))
if not pre:
    choices = (m for m in choices if not Version(m.version).is_prerelease)

best = next(choices, None)
if best is None:
    sys.exit(1)
print(json.dumps(best.as_json()))

"#;

#[derive(Deserialize, Debug)]
struct Match {
    name: String,
    version: String,
}

#[derive(Parser, Debug)]
pub struct ReqExtras {
    /// Install the given package from this git repository
    #[arg(long)]
    git: Option<String>,
    /// Install the given package from this URL
    #[arg(long, conflicts_with = "git", conflicts_with = "path")]
    url: Option<String>,
    /// Install the given package from this local path
    #[arg(long, conflicts_with = "git", conflicts_with = "url")]
    path: Option<PathBuf>,
    /// Force non interpolated absolute paths.
    #[arg(long, requires = "path")]
    absolute: bool,
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
    /// Adds a dependency with a specific feature.
    #[arg(long)]
    features: Vec<String>,
}

impl ReqExtras {
    pub fn force_absolute(&mut self) {
        self.absolute = true;
    }

    pub fn apply_to_requirement(&self, req: &mut Requirement) -> Result<(), Error> {
        if let Some(ref git) = self.git {
            // XXX: today they are all aliases, it might be better to change
            // tag to refs/tags/<tag> and branch to refs/heads/<branch> but
            // this creates some ugly warnings in pip today
            let suffix = match self
                .rev
                .as_ref()
                .or(self.tag.as_ref())
                .or(self.branch.as_ref())
            {
                Some(rev) => format!("@{}", rev),
                None => "".into(),
            };
            req.version_or_url = match req.version_or_url {
                Some(_) => bail!("requirement already has a version marker"),
                None => Some(pep508_rs::VersionOrUrl::Url(
                    format!("git+{}{}", git, suffix).parse().with_context(|| {
                        format!("unable to interpret '{}{}' as git reference", git, suffix)
                    })?,
                )),
            };
        } else if let Some(ref url) = self.url {
            req.version_or_url = match req.version_or_url {
                Some(_) => bail!("requirement already has a version marker"),
                None => Some(pep508_rs::VersionOrUrl::Url(
                    url.parse()
                        .with_context(|| format!("unable to parse '{}' as url", url))?,
                )),
            };
        } else if let Some(ref path) = self.path {
            let file_url = if self.absolute {
                Url::from_file_path(env::current_dir()?.join(path))
                    .map_err(|_| anyhow!("unable to interpret '{}' as path", path.display()))?
            } else {
                let base = env::current_dir()?;
                let rv = pathdiff::diff_paths(base.join(path), &base).ok_or_else(|| {
                    anyhow!(
                        "unable to create relative path from {} to {}",
                        base.display(),
                        path.display()
                    )
                })?;
                Url::from_file_path(Path::new("/${PROJECT_ROOT}").join(rv)).unwrap()
            };
            req.version_or_url = match req.version_or_url {
                Some(_) => bail!("requirement already has a version marker"),
                None => Some(pep508_rs::VersionOrUrl::Url(file_url)),
            };
        }
        for feature in self.features.iter().flat_map(|x| x.split(',')) {
            let feature = feature.trim();
            let extras = req.extras.get_or_insert_with(Vec::new);
            if !extras.iter().any(|x| x == feature) {
                extras.push(feature.into());
            }
        }
        Ok(())
    }
}

/// Adds a Python package to this project.
#[derive(Parser, Debug)]
pub struct Args {
    /// The package to add as PEP 508 requirement string. e.g. 'flask==2.2.3'
    requirements: Vec<String>,
    #[command(flatten)]
    req_extras: ReqExtras,
    /// Add this as dev dependency.
    #[arg(long)]
    dev: bool,
    /// Add this to an optional dependency group.
    #[arg(long, conflicts_with = "dev")]
    optional: Option<String>,
    /// Include pre-releases when finding a package version.
    #[arg(long)]
    pre: bool,
    /// Enables verbose diagnostics.
    #[arg(short, long)]
    verbose: bool,
    /// Turns off all output.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);
    let mut python_path = ensure_self_venv(output).context("error bootstrapping venv")?;
    let mut added = Vec::new();
    python_path.push("bin");
    python_path.push("python");

    let mut pyproject_toml = PyProject::discover()?;
    let py_ver = match pyproject_toml.target_python_version() {
        Some(ver) => ver.format_simple(),
        None => "".to_string(),
    };

    for str_requirement in cmd.requirements {
        let mut requirement = Requirement::from_str(&str_requirement)?;
        cmd.req_extras.apply_to_requirement(&mut requirement)?;

        let mut unearth = Command::new(&python_path);
        unearth
            .arg("-c")
            .arg(PACKAGE_FINDER_SCRIPT)
            .arg(&py_ver)
            .arg(&format_requirement(&requirement).to_string());
        if cmd.pre {
            unearth.arg("--pre");
        }
        let unearth = unearth.stdout(Stdio::piped()).output()?;
        if !unearth.status.success() {
            let log = String::from_utf8_lossy(&unearth.stderr);
            bail!(
                "did not find package {}\n{}",
                format_requirement(&requirement),
                log
            );
        }

        let m: Match = serde_json::from_slice(&unearth.stdout)?;
        if requirement.version_or_url.is_none() {
            requirement.version_or_url = Some(VersionOrUrl::VersionSpecifier(
                VersionSpecifiers::from_str(&format!("~={}", m.version))?,
            ));
        }
        requirement.name = m.name;

        pyproject_toml.add_dependency(
            &requirement,
            if cmd.dev {
                DependencyKind::Dev
            } else if let Some(ref section) = cmd.optional {
                DependencyKind::Optional(section.into())
            } else {
                DependencyKind::Normal
            },
        )?;
        added.push(requirement);
    }

    pyproject_toml.save()?;

    if output != CommandOutput::Quiet {
        for ref requirement in added {
            println!("Added {}", format_requirement(requirement));
        }
    }

    Ok(())
}
