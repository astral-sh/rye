use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;

use anyhow::{anyhow, bail, Context, Error};
use clap::{Parser, ValueEnum};
use pep440_rs::{Operator, Version, VersionSpecifier, VersionSpecifiers};
use pep508_rs::{Requirement, VersionOrUrl};
use serde::Deserialize;
use url::Url;

use crate::bootstrap::ensure_self_venv;
use crate::config::Config;
use crate::consts::VENV_BIN;
use crate::pyproject::{BuildSystem, DependencyKind, ExpandedSources, PyProject};
use crate::sources::py::PythonVersion;
use crate::sync::{autosync, sync, SyncOptions};
use crate::utils::{format_requirement, get_venv_python_bin, set_proxy_variables, CommandOutput};
use crate::uv::UvBuilder;

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
    index_urls=[x[0] for x in sources["index_urls"]],
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

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq)]
enum Pin {
    #[value(alias = "exact", alias = "==", alias = "eq")]
    Equal,
    #[value(alias = "tilde", alias = "compatible", alias = "~=")]
    TildeEqual,
    #[value(alias = ">=", alias = "ge", alias = "gte")]
    GreaterThanEqual,
}

impl From<Pin> for Operator {
    fn from(value: Pin) -> Self {
        match value {
            Pin::Equal => Operator::Equal,
            Pin::TildeEqual => Operator::TildeEqual,
            Pin::GreaterThanEqual => Operator::GreaterThanEqual,
        }
    }
}

impl ReqExtras {
    /// Return true if any path, url, features or similar are set
    /// (anything specific for 1 requirement).
    pub fn has_specifiers(&self) -> bool {
        self.path.is_some() || self.url.is_some() || self.git.is_some() || !self.features.is_empty()
    }

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
            let is_hatchling =
                PyProject::discover()?.build_backend() == Some(BuildSystem::Hatchling);
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
                let mut url = Url::parse("file://")?;
                url.set_path(&Path::new("/${PROJECT_ROOT}").join(rv).to_string_lossy());
                url
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
    #[arg(required = true)]
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
    /// Overrides the pin operator
    #[arg(long)]
    pin: Option<Pin>,
    /// Runs `sync` even if auto-sync is disabled.
    #[arg(long)]
    sync: bool,
    /// Does not run `sync` even if auto-sync is enabled.
    #[arg(long, conflicts_with = "sync")]
    no_sync: bool,
    /// Enables verbose diagnostics.
    #[arg(short, long)]
    verbose: bool,
    /// Turns off all output.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);
    let self_venv = ensure_self_venv(output).context("error bootstrapping venv")?;
    let python_path = self_venv.join(VENV_BIN).join("python");
    let cfg = Config::current();

    let mut pyproject_toml = PyProject::discover()?;
    let py_ver = pyproject_toml.venv_python_version()?;
    let dep_kind = if cmd.dev {
        DependencyKind::Dev
    } else if cmd.excluded {
        DependencyKind::Excluded
    } else if let Some(ref section) = cmd.optional {
        DependencyKind::Optional(section.into())
    } else {
        DependencyKind::Normal
    };
    let default_operator = match cmd.pin {
        Some(pin) => Operator::from(pin),
        None => Config::current().default_dependency_operator(),
    };

    if cmd.req_extras.has_specifiers() && cmd.requirements.len() != 1 {
        bail!("path/url/git/features is not compatible with passing multiple requirements: expected one requirement.")
    }

    let mut requirements = Vec::new();
    for str_requirement in &cmd.requirements {
        let mut requirement = Requirement::from_str(str_requirement)?;
        cmd.req_extras.apply_to_requirement(&mut requirement)?;
        requirements.push(requirement);
    }

    if !cmd.excluded {
        if cfg.use_uv() {
            sync(SyncOptions::python_only().pyproject(None))
                .context("failed to sync ahead of add")?;
            resolve_requirements_with_uv(
                &pyproject_toml,
                &py_ver,
                &mut requirements,
                cmd.pre,
                output,
                &default_operator,
            )?;
        } else {
            for requirement in &mut requirements {
                resolve_requirements_with_unearth(
                    &pyproject_toml,
                    &python_path,
                    &py_ver,
                    requirement,
                    cmd.pre,
                    output,
                    &default_operator,
                )?;
            }
        }
    }

    for requirement in &requirements {
        pyproject_toml.add_dependency(requirement, &dep_kind)?;
    }

    pyproject_toml.save()?;

    if output != CommandOutput::Quiet {
        for ref requirement in requirements {
            echo!(
                "Added {} as {} dependency",
                format_requirement(requirement),
                &dep_kind
            );
        }
    }

    if (cfg.autosync() && !cmd.no_sync) || cmd.sync {
        autosync(&pyproject_toml, output)?;
    }

    Ok(())
}

fn resolve_requirements_with_unearth(
    pyproject_toml: &PyProject,
    python_path: &PathBuf,
    py_ver: &PythonVersion,
    requirement: &mut Requirement,
    pre: bool,
    output: CommandOutput,
    default_operator: &Operator,
) -> Result<(), Error> {
    let matches = find_best_matches_with_unearth(
        pyproject_toml,
        python_path,
        Some(py_ver),
        requirement,
        pre,
    )?;
    if matches.is_empty() {
        let all_matches =
            find_best_matches_with_unearth(pyproject_toml, python_path, None, requirement, pre)
                .unwrap_or_default();
        if all_matches.is_empty() {
            // if we did not consider pre-releases, maybe we could find it by doing so.  In
            // that case give the user a helpful warning before erroring.
            if !pre {
                let all_pre_matches = find_best_matches_with_unearth(
                    pyproject_toml,
                    python_path,
                    None,
                    requirement,
                    true,
                )
                .unwrap_or_default();
                if let Some(pre) = all_pre_matches.into_iter().next() {
                    warn!(
                        "{} ({}) was found considering pre-releases.  Pass --pre to allow use.",
                        pre.name,
                        pre.version.unwrap_or_default()
                    );
                }
                bail!(
                    "did not find package '{}' without using pre-releases.",
                    format_requirement(requirement)
                );
            } else {
                bail!("did not find package '{}'", format_requirement(requirement));
            }
        } else {
            if output != CommandOutput::Quiet {
                echo!("Available package versions:");
                for pkg in all_matches {
                    echo!(
                        "  {} ({}) requires Python {}",
                        pkg.name,
                        pkg.version.unwrap_or_default(),
                        pkg.link
                            .as_ref()
                            .and_then(|x| x.requires_python.as_ref())
                            .map_or("unknown", |x| x as &str)
                    );
                }
                echo!("A possible solution is to raise the version in `requires-python` in `pyproject.toml`.");
            }
            bail!(
                "did not find a version of package '{}' compatible with this version of Python.",
                format_requirement(requirement)
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
                    // local versions or versions with only one component cannot
                    // use ~= but need to use ==.
                    match *default_operator {
                        _ if version.is_local() => Operator::Equal,
                        Operator::TildeEqual if version.release.len() < 2 => {
                            Operator::GreaterThanEqual
                        }
                        ref other => other.clone(),
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
    Ok(())
}

fn find_best_matches_with_unearth(
    pyproject: &PyProject,
    python_path: &PathBuf,
    py_ver: Option<&PythonVersion>,
    requirement: &Requirement,
    pre: bool,
) -> Result<Vec<Match>, Error> {
    let mut unearth = Command::new(python_path);
    let sources = ExpandedSources::from_sources(&pyproject.sources()?)?;

    unearth
        .arg("-c")
        .arg(PACKAGE_FINDER_SCRIPT)
        .arg(match py_ver {
            Some(ver) => ver.format_simple(),
            None => "".into(),
        })
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

fn resolve_requirements_with_uv(
    pyproject_toml: &PyProject,
    py_ver: &PythonVersion,
    requirements: &mut [Requirement],
    pre: bool,
    output: CommandOutput,
    default_operator: &Operator,
) -> Result<(), Error> {
    let venv_path = pyproject_toml.venv_path();
    let py_bin = get_venv_python_bin(&venv_path);
    let sources = ExpandedSources::from_sources(&pyproject_toml.sources()?)?;

    let uv = UvBuilder::new()
        .with_output(output.quieter())
        .with_sources(sources)
        .ensure_exists()?
        .venv(&venv_path, &py_bin, py_ver, None)?;

    for req in requirements {
        let mut new_req = uv.resolve(py_ver, req, pre, env::var("__RYE_UV_EXCLUDE_NEWER").ok())?;

        // if a version or URL is already provided we just use the normalized package name but
        // retain all old information.
        if req.version_or_url.is_some() {
            req.name = new_req.name;
            continue;
        }

        if let Some(ref mut version_or_url) = new_req.version_or_url {
            if let VersionOrUrl::VersionSpecifier(ref mut specs) = version_or_url {
                *version_or_url = VersionOrUrl::VersionSpecifier(VersionSpecifiers::from_iter({
                    let mut new_specs = Vec::new();
                    for spec in specs.iter() {
                        let op = match *default_operator {
                            _ if spec.version().is_local() => Operator::Equal,
                            Operator::TildeEqual if spec.version().release.len() < 2 => {
                                Operator::GreaterThanEqual
                            }
                            ref other => other.clone(),
                        };
                        new_specs.push(
                            VersionSpecifier::new(op, spec.version().clone(), false)
                                .map_err(|msg| anyhow!("invalid version specifier: {}", msg))?,
                        );
                    }
                    new_specs
                }));
            }
        }
        if let Some(old_extras) = &req.extras {
            new_req.extras = Some(old_extras.clone());
        }
        *req = new_req;
    }

    Ok(())
}
