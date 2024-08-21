use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{anyhow, bail, Context, Error};
use clap::{Parser, ValueEnum};
use pep440_rs::{Operator, VersionSpecifier, VersionSpecifiers};
use pep508_rs::{Requirement, VersionOrUrl};
use url::Url;

use crate::bootstrap::ensure_self_venv;
use crate::config::Config;
use crate::lock::KeyringProvider;
use crate::pyproject::{BuildSystem, DependencyKind, ExpandedSources, PyProject};
use crate::sources::py::PythonVersion;
use crate::sync::{autosync, sync, SyncOptions};
use crate::utils::{format_requirement, get_venv_python_bin, CommandOutput};
use crate::uv::UvBuilder;

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
                None => Some(VersionOrUrl::Url(
                    format!("git+{}{}", git, suffix).parse().with_context(|| {
                        format!("unable to interpret '{}{}' as git reference", git, suffix)
                    })?,
                )),
            };
        } else if let Some(ref url) = self.url {
            req.version_or_url = match req.version_or_url {
                Some(_) => bail!("requirement already has a version marker"),
                None => {
                    Some(VersionOrUrl::Url(url.parse().with_context(|| {
                        format!("unable to parse '{}' as url", url)
                    })?))
                }
            };
        } else if let Some(ref path) = self.path {
            // For hatchling build backend, it use {root:uri} for file relative path,
            // but this not supported by uv,
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
                None => Some(VersionOrUrl::Url(file_url)),
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
    #[arg(short, long)]
    dev: bool,
    /// Add this as an excluded dependency that will not be installed even if it's a sub dependency.
    #[arg(long, conflicts_with = "dev", conflicts_with = "optional")]
    excluded: bool,
    /// Add this to an optional dependency group.
    #[arg(long, conflicts_with = "dev", conflicts_with = "excluded")]
    optional: Option<String>,
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

    /// Include pre-releases when finding a package version and automatically syncing the workspace.
    #[arg(long)]
    pre: bool,
    /// Set to `true` to lock with sources in the lockfile when automatically syncing the workspace.
    #[arg(long)]
    with_sources: bool,
    /// Set to `true` to lock with hashes in the lockfile when automatically syncing the workspace.
    #[arg(long)]
    generate_hashes: bool,
    /// Attempt to use `keyring` for authentication for index URLs.
    #[arg(long, value_enum, default_value_t)]
    keyring_provider: KeyringProvider,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);
    ensure_self_venv(output).context("error bootstrapping venv")?;
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
        sync(SyncOptions::python_only().pyproject(None)).context("failed to sync ahead of add")?;
        resolve_requirements_with_uv(
            &pyproject_toml,
            &py_ver,
            &mut requirements,
            cmd.pre,
            output,
            &default_operator,
            cmd.keyring_provider,
        )?;
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
        autosync(
            &pyproject_toml,
            output,
            cmd.pre,
            cmd.with_sources,
            cmd.generate_hashes,
            cmd.keyring_provider,
        )?;
    }

    Ok(())
}

fn resolve_requirements_with_uv(
    pyproject_toml: &PyProject,
    py_ver: &PythonVersion,
    requirements: &mut [Requirement],
    pre: bool,
    output: CommandOutput,
    default_operator: &Operator,
    keyring_provider: KeyringProvider,
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
        let mut new_req = uv.resolve(
            py_ver,
            req,
            pre,
            env::var("__RYE_UV_EXCLUDE_NEWER").ok(),
            keyring_provider,
        )?;

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
