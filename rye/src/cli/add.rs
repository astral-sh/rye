use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;

use anyhow::{anyhow, bail, Context, Error};
use clap::Parser;
use console::style;
use pep440_rs::{Operator, Version, VersionSpecifier, VersionSpecifiers};
use pep508_rs::{Requirement, VersionOrUrl};
use serde::Deserialize;
use url::Url;

use crate::bootstrap::ensure_self_venv;
use crate::consts::VENV_BIN;
use crate::pyproject::{DependencyKind, ExpandedSources, PyProject};
use crate::utils::{format_requirement, set_proxy_variables, CommandOutput};

const PACKAGE_FINDER_SCRIPT: &str = r#"
import sys
import json
from unearth.finder import PackageFinder
from unearth.session import PyPISession
from packaging.version import Version

py_ver = sys.argv[1]
package = sys.argv[2]
sources = json.loads(sys.argv[3])
pre = len(sys.argv) > 4 and sys.argv[4] == "--pre"

finder = PackageFinder(
    index_urls=sources["index_urls"],
    find_links=sources["find_links"],
    trusted_hosts=sources["trusted_hosts"],
)
if py_ver:
    finder.target_python.py_ver = tuple(map(int, py_ver.split('.')))
choices = iter(finder.find_matches(package))
if not pre:
    choices = (m for m in choices if not(m.version and Version(m.version).is_prerelease))

print(json.dumps([x.as_json() for x in choices]))
"#;

#[derive(Deserialize, Debug)]
struct Match {
    name: String,
    version: Option<String>,
    link: Option<Link>,
}

#[derive(Deserialize, Debug)]
struct Link {
    requires_python: Option<String>,
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
            // For hatchling build backend, it use {root:uri} for file relative path,
            // but this not supported by pip-tools,
            // and use ${PROJECT_ROOT} will cause error in hatchling, so force absolute path.
            let is_hatchling = PyProject::discover()?.build_backend().unwrap() == "hatchling";
            let file_url = if self.absolute || is_hatchling {
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
    /// Add this as an excluded dependency that will not be installed even if it's a sub dependency.
    #[arg(long, conflicts_with = "dev", conflicts_with = "optional")]
    excluded: bool,
    /// Add this to an optional dependency group.
    #[arg(long, conflicts_with = "dev", conflicts_with = "excluded")]
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
    python_path.push(VENV_BIN);
    python_path.push("python");

    let mut pyproject_toml = PyProject::discover()?;
    let py_ver = match pyproject_toml.target_python_version() {
        Some(ver) => ver.format_simple(),
        None => "".to_string(),
    };
    let dep_kind = if cmd.dev {
        DependencyKind::Dev
    } else if cmd.excluded {
        DependencyKind::Excluded
    } else if let Some(ref section) = cmd.optional {
        DependencyKind::Optional(section.into())
    } else {
        DependencyKind::Normal
    };

    for str_requirement in cmd.requirements {
        let mut requirement = Requirement::from_str(&str_requirement)?;
        cmd.req_extras.apply_to_requirement(&mut requirement)?;

        // if we are excluding, we do not want a specific dependency version
        // stored, so we just skip the unearth step
        if !cmd.excluded {
            let matches = find_best_matches(
                &pyproject_toml,
                &python_path,
                Some(&py_ver),
                &requirement,
                cmd.pre,
            )?;
            if matches.is_empty() {
                let all_matches =
                    find_best_matches(&pyproject_toml, &python_path, None, &requirement, cmd.pre)
                        .unwrap_or_default();
                if all_matches.is_empty() {
                    // if we did not consider pre-releases, maybe we could find it by doing so.  In
                    // that case give the user a helpful warning before erroring.
                    if !cmd.pre {
                        let all_pre_matches = find_best_matches(
                            &pyproject_toml,
                            &python_path,
                            None,
                            &requirement,
                            true,
                        )
                        .unwrap_or_default();
                        if let Some(pre) = all_pre_matches.into_iter().next() {
                            eprintln!(
                                "{}: {} ({}) was found considering pre-releases.  Pass --pre to allow use.",
                                style("warning").red(),
                                pre.name,
                                pre.version.unwrap_or_default()
                            );
                        }
                        bail!(
                            "did not find package '{}' without using pre-releases.",
                            format_requirement(&requirement)
                        );
                    } else {
                        bail!(
                            "did not find package '{}'",
                            format_requirement(&requirement)
                        );
                    }
                } else {
                    if output != CommandOutput::Quiet {
                        eprintln!("Available package versions:");
                        for pkg in all_matches {
                            eprintln!(
                                "  {} ({}) requires Python {}",
                                pkg.name,
                                pkg.version.unwrap_or_default(),
                                pkg.link
                                    .as_ref()
                                    .and_then(|x| x.requires_python.as_ref())
                                    .map_or("unknown", |x| x as &str)
                            );
                        }
                        eprintln!("A possible solution is to raise the version in `requires-python` in `pyproject.toml`.");
                    }
                    bail!(
                        "did not find a version of package '{}' compatible with this version of Python.",
                        format_requirement(&requirement)
                    );
                }
            }

            let m = matches.into_iter().next().unwrap();
            if m.version.is_some() && requirement.version_or_url.is_none() {
                let version = Version::from_str(m.version.as_ref().unwrap())
                    .map_err(|msg| anyhow!("invalid version: {}", msg))?;
                requirement.version_or_url = Some(VersionOrUrl::VersionSpecifier(
                    VersionSpecifiers::from_iter(Some(
                        VersionSpecifier::new(
                            if version.is_local() {
                                Operator::Equal
                            } else {
                                Operator::TildeEqual
                            },
                            Version::from_str(m.version.as_ref().unwrap())
                                .map_err(|msg| anyhow!("invalid version: {}", msg))?,
                            false,
                        )
                        .map_err(|msg| anyhow!("invalid version specifier: {}", msg))?,
                    )),
                ));
            }
            requirement.name = m.name;
        }

        pyproject_toml.add_dependency(&requirement, &dep_kind)?;
        added.push(requirement);
    }

    pyproject_toml.save()?;

    if output != CommandOutput::Quiet {
        for ref requirement in added {
            println!(
                "Added {} as {} dependency",
                format_requirement(requirement),
                &dep_kind
            );
        }
    }

    Ok(())
}

fn find_best_matches(
    pyproject: &PyProject,
    python_path: &PathBuf,
    py_ver: Option<&str>,
    requirement: &Requirement,
    pre: bool,
) -> Result<Vec<Match>, Error> {
    let mut unearth = Command::new(python_path);
    let sources = ExpandedSources::from_sources(&pyproject.sources()?)?;

    unearth
        .arg("-c")
        .arg(PACKAGE_FINDER_SCRIPT)
        .arg(py_ver.unwrap_or(""))
        .arg(&format_requirement(requirement).to_string())
        .arg(serde_json::to_string(&sources)?);
    if pre {
        unearth.arg("--pre");
    }
    set_proxy_variables(&mut unearth);
    let unearth = unearth.stdout(Stdio::piped()).output()?;
    if unearth.status.success() {
        Ok(serde_json::from_slice(&unearth.stdout)?)
    } else {
        let log = String::from_utf8_lossy(&unearth.stderr);
        bail!(
            "failed to resolve package {}\n{}",
            format_requirement(requirement),
            log
        );
    }
}
