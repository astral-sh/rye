use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::{env, fs};

use anyhow::{anyhow, bail, Context, Error};
use clap::Parser;
use configparser::ini::Ini;
use console::style;
use license::License;
use minijinja::{context, Environment};
use monotrail_utils::RequirementsTxt;
use pep440_rs::VersionSpecifier;
use pep508_rs::Requirement;
use serde_json::Value;
use tempfile::tempdir;

use crate::bootstrap::ensure_self_venv;
use crate::config::Config;
use crate::platform::{
    get_default_author_with_fallback, get_latest_cpython_version,
    get_python_version_request_from_pyenv_pin,
};
use crate::pyproject::BuildSystem;
use crate::sources::PythonVersionRequest;
use crate::utils::{
    copy_dir, escape_string, format_requirement, get_venv_python_bin, is_inside_git_work_tree,
    CommandOutput, CopyDirOptions,
};

/// Creates a new python project.
#[derive(Parser, Debug)]
pub struct Args {
    /// Where to place the project (defaults to current path)
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Minimal Python version supported by this project.
    #[arg(long)]
    min_py: Option<String>,
    /// Python version to use for the virtualenv.
    #[arg(short, long)]
    py: Option<String>,
    /// Do not create a readme.
    #[arg(long)]
    no_readme: bool,
    /// Do not create .python-version file (requires-python will be used)
    #[arg(long)]
    no_pin: bool,
    /// Which build system should be used(defaults to hatchling)?
    #[arg(long)]
    build_system: Option<BuildSystem>,
    /// Which license should be used (SPDX identifier)?
    #[arg(long)]
    license: Option<String>,
    /// The name of the package.
    #[arg(long)]
    name: Option<String>,
    /// Set "Private :: Do Not Upload" classifier, used for private projects
    #[arg(long)]
    private: bool,
    /// Don't import from setup.cfg, setup.py, or requirements files.
    #[arg(long)]
    no_import: bool,
    /// Requirements files to initialize pyproject.toml with.
    #[arg(short, long, name = "REQUIREMENTS_FILE", conflicts_with = "no_import")]
    requirements: Option<Vec<PathBuf>>,
    /// Development requirements files to initialize pyproject.toml with.
    #[arg(long, name = "DEV_REQUIREMENTS_FILE", conflicts_with = "no_import")]
    dev_requirements: Option<Vec<PathBuf>>,
    /// Enables verbose diagnostics.
    #[arg(short, long)]
    verbose: bool,
    /// Turns off all output.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
}

/// The pyproject.toml template
///
/// This uses a template just to simplify the flexibility of emitting it.
const TOML_TEMPLATE: &str = r#"[project]
name = {{ name }}
version = {{ version }}
description = {{ description }}
{%- if author %}
authors = [
    { name = {{ author[0] }}, email = {{ author[1] }} }
]
{%- endif %}
{%- if dependencies %}
dependencies = [
{%- for dependency in dependencies %}
    {{ dependency }},
{%- endfor %}
]
{%- else %}
dependencies = []
{%- endif %}
{%- if with_readme %}
readme = "README.md"
{%- endif %}
requires-python = {{ requires_python }}
{%- if license %}
license = { text = {{ license }} }
{%- endif %}
{%- if private %}
classifiers = ["Private :: Do Not Upload"]
{%- endif %}

[build-system]
{%- if build_system == "hatchling" %}
requires = ["hatchling"]
build-backend = "hatchling.build"
{%- elif build_system == "setuptools" %}
requires = ["setuptools>=61.0"]
build-backend = "setuptools.build_meta"
{%- elif build_system == "flit" %}
requires = ["flit_core>=3.4"]
build-backend = "flit_core.buildapi"
{%- elif build_system == "pdm" %}
requires = ["pdm-backend"]
build-backend = "pdm.backend"
{%- endif %}

[tool.rye]
managed = true
{%- if dev_dependencies %}
dev-dependencies = [
{%- for dependency in dev_dependencies %}
    {{ dependency }},
{%- endfor %}
]
{%- else %}
dev-dependencies = []
{%- endif %}

{%- if build_system == "hatchling" %}

[tool.hatch.metadata]
allow-direct-references = true
{%- endif %}

"#;

/// The template for the readme file.
const README_TEMPLATE: &str = r#"# {{ name }}

Describe your project here.

{%- if license %}
* License: {{ license }}
{%- endif %}

"#;

const LICENSE_TEMPLATE: &str = r#"
{{ license_text }}
"#;

/// Template for the __init__.py
const INIT_PY_TEMPLATE: &str = r#"def hello():
    return "Hello from {{ name }}!"

"#;

/// Template for fresh gitignore files
const GITIGNORE_TEMPLATE: &str = r#"# python generated files
__pycache__/
*.py[oc]
build/
dist/
wheels/
*.egg-info

# venv
.venv

"#;

/// Script used for setup.py setup proxy.
const SETUP_PY_PROXY_SCRIPT: &str = r#"
import json, sys
from pathlib import Path
from tempfile import TemporaryDirectory

def setup(**kwargs) -> None:
    print(json.dumps(kwargs), file=sys.stderr)

if __name__ == "setuptools":
    _setup_proxy_module = sys.modules.pop("setuptools")
    _setup_proxy_cwd = sys.path.pop(0)
    import setuptools as __setuptools
    sys.path.insert(0, _setup_proxy_cwd)
    sys.modules["setuptools"] = _setup_proxy_module
    def __getattr__(name):
        return getattr(__setuptools, name)
"#;

pub fn execute(cmd: Args) -> Result<(), Error> {
    let cfg = Config::current();
    let env = Environment::new();
    let dir = env::current_dir()?.join(cmd.path);
    let toml = dir.join("pyproject.toml");
    let readme = dir.join("README.md");
    let license_file = dir.join("LICENSE.txt");
    let python_version_file = dir.join(".python-version");

    if toml.is_file() {
        bail!("pyproject.toml already exists");
    }

    // fail silently if it already exists or cannot be created.
    fs::create_dir_all(&dir).ok();

    // Write pyproject.toml
    let mut requires_python = match cmd.min_py {
        Some(py) => format!(">= {}", py),
        None => get_python_version_request_from_pyenv_pin(&dir)
            .map(|x| format!(">= {}.{}", x.major, x.minor.unwrap_or_default()))
            .unwrap_or_else(|| cfg.default_requires_python()),
    };
    let py = match cmd.py {
        Some(py) => PythonVersionRequest::from_str(&py)
            .map_err(|msg| anyhow!("invalid version: {}", msg))?,
        None => match get_python_version_request_from_pyenv_pin(&dir) {
            Some(ver) => ver,
            None => PythonVersionRequest::from(get_latest_cpython_version()?),
        },
    };
    if !cmd.no_pin
        && !VersionSpecifier::from_str(&requires_python)
            .map_err(|msg| anyhow!("invalid version specifier: {}", msg))?
            .contains(&py.clone().into())
    {
        warn!("conflicted python version with project's requires-python, will auto fix it.");
        requires_python = format!(">= {}.{}", py.major, py.minor.unwrap_or_default());
    }

    // In some cases there might not be a file name (eg: docker root)
    let name = slug::slugify(cmd.name.unwrap_or_else(|| {
        dir.file_name()
            .map(|x| x.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".into())
    }));
    let version = "0.1.0";
    let author = get_default_author_with_fallback();
    let license = match cmd.license {
        Some(license) => Some(license),
        None => cfg.default_license(),
    };
    if license.is_some() && !license_file.is_file() {
        let license_obj: &dyn License = license
            .clone()
            .unwrap()
            .parse()
            .expect("current license not an valid license id");
        let license_text = license_obj.text();
        let rv = env.render_named_str(
            "LICENSE.txt",
            LICENSE_TEMPLATE,
            context! {
                license_text,
            },
        )?;
        fs::write(&license_file, rv)?;
    }

    // initialize with no metadata
    let mut metadata = Metadata::new();

    // by default rye attempts to import metadata first.
    if !cmd.no_import {
        let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);
        let options = ImportOptions {
            output,
            requirements: cmd.requirements,
            dev_requirements: cmd.dev_requirements,
            ..Default::default()
        };
        try_import_project_metadata(&mut metadata, &dir, options)?;
    }

    let imported_something = metadata.name.is_some() || metadata.dependencies.is_some();

    // if we're missing metadata after the import we update it with what's found from normal initialization.
    if metadata.name.is_none() {
        metadata.name = Some(name);
    }
    if metadata.version.is_none() {
        metadata.version = Some(version.to_string());
    }
    if metadata.description.is_none() {
        metadata.description = Some("Add your description here".to_string())
    }
    if metadata.author.is_none() {
        metadata.author = author;
    }
    if metadata.requires_python.is_none() {
        metadata.requires_python = Some(requires_python);
    }
    if metadata.license.is_none() {
        metadata.license = license;
    }
    if metadata.dependencies.is_none() {
        metadata.dependencies = Some(Vec::new())
    }

    // write .python-version
    if !cmd.no_pin && !python_version_file.is_file() {
        fs::write(python_version_file, format!("{}\n", py))
            .context("could not write .python-version file")?;
    }

    // create a readme if one is missing
    let with_readme = if readme.is_file() {
        true
    } else if !cmd.no_readme {
        let rv = env.render_named_str(
            "README.txt",
            README_TEMPLATE,
            context! {
                name => metadata.name,
                license => metadata.license,
            },
        )?;
        fs::write(&readme, rv)?;
        true
    } else {
        false
    };

    let build_system = match cmd.build_system {
        Some(build_system) => build_system,
        None => cfg.default_build_system().unwrap_or(BuildSystem::Hatchling),
    };

    let private = cmd.private;

    let rv = env.render_named_str(
        "pyproject.json",
        TOML_TEMPLATE,
        context! {
            name => metadata.name,
            description => metadata.description,
            version => metadata.version,
            author => metadata.author,
            requires_python => metadata.requires_python,
            license => metadata.license,
            dependencies => metadata.dependencies,
            dev_dependencies => metadata.dev_dependencies,
            with_readme,
            build_system,
            private,
        },
    )?;
    fs::write(&toml, rv).context("failed to write pyproject.toml")?;

    let src_dir = dir.join("src");
    if !imported_something && !src_dir.is_dir() {
        let name = metadata.name.expect("project name");
        let project_dir = src_dir.join(name.replace('-', "_"));
        fs::create_dir_all(&project_dir).ok();
        let rv = env.render_named_str("__init__.py", INIT_PY_TEMPLATE, context! { name })?;
        fs::write(project_dir.join("__init__.py"), rv).context("failed to write __init__.py")?;
    }

    // if git init is successful prepare the local git repository
    if !is_inside_git_work_tree(&dir)
        && Command::new("git")
            .arg("init")
            .current_dir(&dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    {
        let gitignore = dir.join(".gitignore");

        // create a .gitignore if one is missing
        if !gitignore.is_file() {
            let rv = env.render_named_str("gitignore.txt", GITIGNORE_TEMPLATE, ())?;
            fs::write(&gitignore, rv).context("failed to write .gitignore")?;
        }
    }

    echo!(
        "{} Initialized project in {}",
        style("success:").green(),
        dir.display()
    );
    echo!("  Run `rye sync` to get started");

    Ok(())
}

#[derive(Default)]
struct Metadata {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    author: Option<(String, String)>,
    requires_python: Option<String>,
    license: Option<String>,
    dependencies: Option<Vec<String>>,
    dev_dependencies: Option<Vec<String>>,
}

impl Metadata {
    fn new() -> Self {
        Self::default()
    }
}

struct ImportOptions {
    output: CommandOutput,
    setup_py_name: String,
    setup_cfg_name: String,
    requirements: Option<Vec<PathBuf>>,
    dev_requirements: Option<Vec<PathBuf>>,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            output: Default::default(),
            setup_py_name: "setup.py".to_string(),
            setup_cfg_name: "setup.cfg".to_string(),
            requirements: None,
            dev_requirements: None,
        }
    }
}

/// Attempt to import data from setup.py, setup.cfg, and requirements files if metadata is missing.
fn try_import_project_metadata(
    metadata: &mut Metadata,
    from: impl AsRef<Path>,
    options: ImportOptions,
) -> Result<&mut Metadata, Error> {
    let dir = from.as_ref();
    let setup_cfg = dir.join(options.setup_cfg_name);
    let setup_py = dir.join(options.setup_py_name);
    let mut requirements = BTreeMap::new();
    let mut dev_requirements = BTreeMap::new();

    // if a setup.py or setup.cfg are found we attempt an import, only importing
    // what our metadata is missing
    if setup_py.is_file() {
        // TODO(cnpryer): May need to be smarter with what Python version is used
        let python = get_venv_python_bin(
            &ensure_self_venv(options.output).context("error bootstrapping venv")?,
        );
        import_setup_py(metadata, &mut requirements, &setup_py, &python)?;
    }
    if setup_cfg.is_file() {
        import_setup_cfg(metadata, &mut requirements, &setup_cfg)?;
    }

    if let Some(paths) = options.requirements {
        for p in paths {
            import_requirements_file(&mut requirements, p)?;
        }
    }
    if let Some(paths) = options.dev_requirements {
        for p in paths {
            import_requirements_file(&mut dev_requirements, p)?;
        }
    }
    if metadata.dependencies.is_none() && !requirements.is_empty() {
        metadata.dependencies = Some(requirements.into_values().collect());
    }
    if metadata.dev_dependencies.is_none() && !dev_requirements.is_empty() {
        metadata.dev_dependencies = Some(dev_requirements.into_values().collect());
    }

    Ok(metadata)
}

fn import_setup_py<T: AsRef<Path>>(
    metadata: &mut Metadata,
    requirements: &mut BTreeMap<String, String>,
    path: T,
    python: T,
) -> Result<(), Error> {
    let json = get_setup_py_json(path, python)?;
    if let Some(Value::String(name)) = json.get("name") {
        if metadata.name.is_none() {
            metadata.name = Some(name.to_string());
        }
    }
    if let Some(Value::String(version)) = json.get("version") {
        if metadata.version.is_none() {
            metadata.version = Some(version.to_string());
        }
    }
    if let Some(Value::String(description)) = json.get("description") {
        if metadata.description.is_none() {
            metadata.description = Some(description.to_string());
        }
    }
    if let Some(Value::String(author)) = json.get("author") {
        if metadata.author.is_none() {
            metadata.author = Some((
                author.to_string(),
                json.get("author_email")
                    .map(ToString::to_string)
                    .map(escape_string)
                    .unwrap_or_else(String::new),
            ));
        }
    }
    if let Some(Value::String(python_requires)) = json.get("python_requires") {
        if metadata.requires_python.is_none() {
            metadata.requires_python = Some(python_requires.to_string());
        }
    }
    if let Some(Value::String(license)) = json.get("license") {
        if metadata.license.is_none() {
            metadata.license = Some(license.to_string());
        }
    }
    if let Some(Value::Array(reqs)) = json.get("install_requires") {
        reqs.iter()
            .map(ToString::to_string)
            .map(escape_string)
            .filter_map(|x| Requirement::from_str(&x).ok())
            .for_each(|x| {
                requirements.insert(x.name.to_string(), format_requirement(&x).to_string());
            });
    }
    Ok(())
}

fn import_setup_cfg(
    metadata: &mut Metadata,
    requirements: &mut BTreeMap<String, String>,
    path: impl AsRef<Path>,
) -> Result<(), Error> {
    let mut ini = Ini::new();
    ini.set_multiline(true);
    let config = ini.load(path).map_err(|msg| anyhow::anyhow!(msg))?;
    if let Some(section) = config.get("metadata") {
        if metadata.name.is_none() {
            if let Some(Some(name)) = section.get("name") {
                metadata.name = Some(name.to_string());
            }
        }
        if metadata.version.is_none() {
            if let Some(Some(version)) = section.get("version") {
                metadata.version = Some(version.to_string());
            }
        }
        if metadata.description.is_none() {
            if let Some(Some(description)) = section.get("description") {
                metadata.description = Some(description.to_string());
            }
        }
        if metadata.author.is_none() {
            if let Some(Some(author)) = section.get("author") {
                let email = match section.get("author_email") {
                    Some(Some(it)) => it,
                    _ => "",
                };
                metadata.author = Some((author.to_string(), email.to_string()));
            }
        }
        if metadata.license.is_none() {
            if let Some(Some(license)) = section.get("license") {
                metadata.license = Some(license.to_string());
            }
        }
    }
    if let Some(section) = config.get("options") {
        if metadata.requires_python.is_none() {
            if let Some(Some(python_requires)) = section.get("python_requires") {
                metadata.requires_python = Some(python_requires.to_string());
            }
        }
        if let Some(Some(reqs)) = section.get("install_requires") {
            reqs.lines()
                .filter_map(|x| Requirement::from_str(x).ok())
                .for_each(|x| {
                    requirements.insert(x.name.to_string(), format_requirement(&x).to_string());
                });
        }
    }
    Ok(())
}

fn get_setup_py_json<T: AsRef<Path>>(path: T, python: T) -> Result<Value, Error> {
    let python = python.as_ref();
    let setup_py = path.as_ref();
    let temp_dir = tempdir()?;
    let dir = setup_py
        .parent()
        .context("could not establish setup.py parent dir")?;

    let options = CopyDirOptions {
        exclude: vec![dir.join(".git"), dir.join(".tox")],
    };
    copy_dir(dir, temp_dir.path(), &options)?;

    let setuptools_proxy = temp_dir.path().join("setuptools.py");
    fs::write(setuptools_proxy, SETUP_PY_PROXY_SCRIPT)?;

    let cmd = Command::new(python)
        .arg(setup_py)
        .env("PYTHONPATH", temp_dir.path())
        .stderr(Stdio::piped())
        .output()?;
    if cmd.status.success() {
        Ok(serde_json::from_slice(&cmd.stderr)?)
    } else {
        let log = String::from_utf8_lossy(&cmd.stderr);
        bail!("failed to proxy setup.py\n{}", log);
    }
}

/// Import from requirements files.
///
/// Unsupported as of monotrail-utils v0.0.1:
///  * `-e <path>`. TBD
///  * `<path>`. TBD
///  * `<archive_url>`. TBD
///  * Options without a requirement, such as `--find-links` or `--index-url`
///
/// See https://github.com/mitsuhiko/rye/issues/191
fn import_requirements_file(
    requirements: &mut BTreeMap<String, String>,
    path: impl AsRef<Path>,
) -> Result<(), Error> {
    let path = path.as_ref();
    let dir = path
        .parent()
        .context("could not establish setup.py parent dir")?;
    let data = RequirementsTxt::parse(path, dir)?;
    data.requirements.iter().for_each(|x| {
        requirements
            .entry(x.requirement.name.to_string())
            .or_insert(format_requirement(&x.requirement).to_string());
    });
    Ok(())
}
