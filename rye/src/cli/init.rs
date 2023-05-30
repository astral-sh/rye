use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::{env, fs};

use anyhow::{anyhow, bail, Context, Error};
use clap::Parser;
use console::style;
use ini::Ini;
use license::License;
use minijinja::{context, Environment};
use pep440_rs::VersionSpecifier;
use serde_json::Value;
use tempfile::tempdir;

use crate::config::Config;
use crate::platform::{
    get_default_author, get_latest_cpython_version, get_python_version_request_from_pyenv_pin,
    get_toolchain_python_bin,
};
use crate::pyproject::BuildSystem;
use crate::sources::PythonVersionRequest;
use crate::utils::is_inside_git_work_tree;

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
    /// Import a project with a setup.cfg, setup.py, or requirements files.
    #[arg(long)]
    import: bool,
    /// Requirements files to initialize pyproject.toml with.
    #[arg(short, long, name = "requirements_file")]
    requirements: Option<Vec<PathBuf>>,
    /// Development requirements files to initialize pyproject.toml with.
    #[arg(long, name = "dev_requirements_file")]
    dev_requirements: Option<Vec<PathBuf>>,
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
dependencies = []
{%- if with_readme %}
readme = "README.md"
{%- endif %}
requires-python = {{ requires_python }}
{%- if license %}
license = { text = {{ license }} }
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
    del _setup_proxy_module
    del _setup_proxy_cwd
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
        None => get_python_version_request_from_pyenv_pin()
            .map(|x| format!(">= {}.{}", x.major, x.minor.unwrap_or_default()))
            .unwrap_or_else(|| cfg.default_requires_python()),
    };
    let py = match cmd.py {
        Some(py) => PythonVersionRequest::from_str(&py)
            .map_err(|msg| anyhow!("invalid version: {}", msg))?,
        None => match get_python_version_request_from_pyenv_pin() {
            Some(ver) => ver,
            None => PythonVersionRequest::from(get_latest_cpython_version()?),
        },
    };
    if !cmd.no_pin
        && !VersionSpecifier::from_str(&requires_python)
            .map_err(|msg| anyhow!("invalid version specifier: {}", msg))?
            .contains(&py.clone().into())
    {
        eprintln!(
            "{} conflicted python version with project's requires-python, will auto fix it.",
            style("warning:").red()
        );
        requires_python = format!(">= {}.{}", py.major, py.minor.unwrap_or_default());
    }

    let name = slug::slugify(
        cmd.name
            .unwrap_or_else(|| dir.file_name().unwrap().to_string_lossy().into_owned()),
    );
    let version = "0.1.0";
    let author = get_default_author();
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

    let mut metadata = Metadata {
        name,
        version: version.to_string(),
        description: "Add your description here".to_string(),
        author,
        requires_python: Some(requires_python),
        license,
        dependencies: None,
        dev_dependencies: None,
    };
    if cmd.import {
        // TODO(cnpryer): May need to be smarter with what Python version is used
        let python = get_toolchain_python_bin(&get_latest_cpython_version()?)?;
        import_project_metadata(&mut metadata, &dir, &python, None, None)?;
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
            with_readme,
            build_system,
        },
    )?;
    fs::write(&toml, rv).context("failed to write pyproject.toml")?;

    let src_dir = dir.join("src");
    if !src_dir.is_dir() {
        let project_dir = src_dir.join(metadata.name.replace('-', "_"));
        fs::create_dir_all(&project_dir).ok();
        let rv = env.render_named_str(
            "__init__.py",
            INIT_PY_TEMPLATE,
            context! { name => metadata.name },
        )?;
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

    eprintln!(
        "{} Initialized project in {}",
        style("success:").green(),
        dir.display()
    );
    eprintln!("  Run `rye sync` to get started");

    Ok(())
}

#[derive(Default)]
struct Metadata {
    name: String,
    version: String,
    description: String,
    author: Option<(String, String)>,
    requires_python: Option<String>,
    license: Option<String>,
    dependencies: Option<Vec<String>>,
    dev_dependencies: Option<Vec<String>>,
}

fn import_project_metadata<T: AsRef<Path>>(
    metadata: &mut Metadata,
    dir: T,
    python: T,
    requirements_files: Option<Vec<PathBuf>>,
    dev_requirements_files: Option<Vec<PathBuf>>,
) -> Result<&mut Metadata, Error> {
    let dir = dir.as_ref();
    let python = python.as_ref();
    let setup_cfg = dir.join("setup.cfg");
    let setup_py = dir.join("setup.py");

    // TODO(cnpryer): Start with setup.py import and then selectively import from cfg
    if let Ok(ini) = Ini::load_from_file(setup_cfg) {
        if let Some(section) = ini.section(Some("metadata")) {
            if let Some(name) = section.get("name") {
                metadata.name = name.to_string();
            }
            if let Some(version) = section.get("version") {
                metadata.version = version.to_string();
            }
            if let Some(description) = section.get("description") {
                metadata.description = description.to_string();
            }
            if let Some(it) = section.get("author") {
                metadata.author = Some((
                    it.to_string(),
                    section.get("author_email").unwrap_or("").to_string(),
                ));
            }
            metadata.license = section.get("license").map(|x| x.to_string());
        }
        if let Some(section) = ini.section(Some("options")) {
            metadata.requires_python = section.get("requires_python").map(|x| x.to_string());
            if let Some(reqs) = section.get("install_requires") {
                metadata.dependencies = Some(reqs.lines().map(|x| x.to_string()).collect());
            }
        }
    }

    if let Ok(json) = get_setup_py_json(setup_py.as_path(), python) {
        if let Some(name) = json.get("name") {
            metadata.name = name.to_string();
        }
        if let Some(version) = json.get("version") {
            metadata.version = version.to_string();
        }
        if let Some(description) = json.get("description") {
            metadata.description = description.to_string();
        }
        if let Some(it) = json.get("author") {
            metadata.author = Some((
                it.to_string(),
                json.get("author_email")
                    .map(|x| x.to_string())
                    .unwrap_or_else(String::new),
            ));
        }
        if let Some(requires_python) = json.get("requires_python") {
            metadata.requires_python = Some(requires_python.to_string());
        }
        if let Some(license) = json.get("license") {
            metadata.license = Some(license.to_string());
        }
        if let Some(Value::Array(reqs)) = json.get("install_requires") {
            metadata.dependencies = Some(
                reqs.iter()
                    .filter_map(|value| value.as_str().map(String::from))
                    .collect::<Vec<String>>(),
            );
        }
    }

    if let Some(paths) = requirements_files {
        for p in paths {
            if let Ok(deps) = parse_requirements_file(p) {
                if let Some(x) = metadata.dependencies.as_mut() {
                    x.extend(deps)
                }
            }
        }
    }
    if let Some(paths) = dev_requirements_files {
        for p in paths {
            if let Ok(deps) = parse_requirements_file(p) {
                if let Some(x) = metadata.dev_dependencies.as_mut() {
                    x.extend(deps)
                }
            }
        }
    }
    if let Some(x) = metadata.dependencies.as_mut() {
        x.dedup()
    }
    if let Some(x) = metadata.dev_dependencies.as_mut() {
        x.dedup()
    }

    Ok(metadata)
}

fn get_setup_py_json<T: AsRef<Path>>(path: T, python: T) -> Result<Value, Error> {
    let python = python.as_ref();
    let setup_py = path.as_ref();
    let temp_dir = tempdir().unwrap();
    let dir = setup_py
        .parent()
        .context("could not establish setup.py parent dir")?;
    copy_dir(dir, temp_dir.path()).unwrap();

    let setuptools_proxy = temp_dir.path().join("setuptools.py");
    fs::write(setuptools_proxy, SETUP_PY_PROXY_SCRIPT).unwrap();

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

fn parse_requirements_file<T: AsRef<Path>>(_path: T) -> Result<Vec<String>, Error> {
    todo!()
}

// TODO(cnpryer)
pub fn copy_dir<T: AsRef<Path>>(from: T, to: T) -> Result<(), Error> {
    let (from, to) = (from.as_ref(), to.as_ref());
    let mut stack = Vec::new();
    stack.push(PathBuf::from(from));
    let target_root = to.to_path_buf();
    let from_component_count = from.to_path_buf().components().count();
    while let Some(working_path) = stack.pop() {
        // Collects the trailing components of the path
        let src: PathBuf = working_path
            .components()
            .skip(from_component_count)
            .collect();
        let dest = if src.components().count() == 0 {
            target_root.clone()
        } else {
            target_root.join(&src)
        };
        if !dest.exists() {
            fs::create_dir_all(&dest).expect("to create dir");
        }
        for entry in fs::read_dir(working_path).expect("to read dir") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if let Some(filename) = path.file_name() {
                fs::copy(&path, dest.join(filename)).expect("to copy");
            }
        }
    }

    Ok(())
}
