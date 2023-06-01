use std::collections::HashMap;
use std::fs::File;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::{env, fs, io};

use anyhow::{anyhow, bail, Context, Error};
use clap::Parser;
use configparser::ini::Ini;
use console::style;
use license::License;
use minijinja::{context, Environment};
use pep440_rs::VersionSpecifier;
use pep508_rs::Requirement;
use serde_json::Value;
use tempfile::tempdir;

use crate::config::Config;
use crate::platform::{
    get_default_author, get_latest_cpython_version, get_python_version_request_from_pyenv_pin,
    get_toolchain_python_bin,
};
use crate::pyproject::BuildSystem;
use crate::sources::PythonVersionRequest;
use crate::utils::{escape_string, is_inside_git_work_tree};

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
    #[arg(short, long, name = "REQUIREMENTS_FILE")]
    requirements: Option<Vec<PathBuf>>,
    /// Development requirements files to initialize pyproject.toml with.
    #[arg(long, name = "DEV_REQUIREMENTS_FILE")]
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
dependencies = {{ dependencies }}
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
{%- if dev_dependencies is not none %}
dev-dependencies = {{ dev_dependencies }}
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

    // In some cases there might not be a file name (eg: docker root)
    let name = slug::slugify(cmd.name.unwrap_or_else(|| {
        dir.file_name()
            .map(|x| x.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".into())
    }));
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
        dependencies: Some(Vec::new()),
        dev_dependencies: None,
    };

    // Attempt to import metadata if --import is used or setup files are available.
    if cmd.import || import_available(&dir) {
        // TODO(cnpryer): May need to be smarter with what Python version is used
        let python = get_toolchain_python_bin(&get_latest_cpython_version()?)?;
        import_project_metadata(
            &mut metadata,
            &dir,
            &python,
            cmd.requirements,
            cmd.dev_requirements,
        )?;
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
            dependencies => metadata.dependencies,
            dev_dependencies => metadata.dev_dependencies,
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

/// Check if setup.py or setup.cfg exist.
fn import_available<T: AsRef<Path>>(dir: T) -> bool {
    let dir = dir.as_ref();
    dir.join("setup.py").is_file() || dir.join("setup.cfg").is_file()
}

/// Pull importable data from setup.py, setup.cfg, and requirement files.
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
    let mut requirements = HashMap::new();
    let mut dev_requirements = HashMap::new();

    // TODO(cnpryer): Start with setup.py import and then selectively import from cfg
    if setup_cfg.is_file() {
        let mut ini = Ini::new();
        ini.set_multiline(true);
        let config = ini.load(setup_cfg).map_err(|msg| anyhow::anyhow!(msg))?;
        if let Some(section) = config.get("metadata") {
            if let Some(Some(name)) = section.get("name") {
                metadata.name = name.to_string();
            }
            if let Some(Some(version)) = section.get("version") {
                metadata.version = version.to_string();
            }
            if let Some(Some(description)) = section.get("description") {
                metadata.description = description.to_string();
            }
            if let Some(Some(author)) = section.get("author") {
                let email = match section.get("author_email") {
                    Some(Some(it)) => it,
                    _ => "",
                };
                metadata.author = Some((author.to_string(), email.to_string()));
            }
            if let Some(Some(license)) = section.get("license") {
                metadata.license = Some(license.to_string());
            }
        }
        if let Some(section) = config.get("options") {
            if let Some(Some(requires_python)) = section.get("requires_python") {
                metadata.requires_python = Some(requires_python.to_string());
            }
            if let Some(Some(reqs)) = section.get("install_requires") {
                reqs.lines()
                    .filter_map(|x| Requirement::from_str(x).ok())
                    .for_each(|x| {
                        requirements.insert(x.name.to_string(), x.to_string());
                    });
            }
        }
    }

    if let Ok(json) = get_setup_py_json(setup_py.as_path(), python) {
        if let Some(Value::String(name)) = json.get("name") {
            metadata.name = name.to_string();
        }
        if let Some(Value::String(version)) = json.get("version") {
            metadata.version = version.to_string();
        }
        if let Some(Value::String(description)) = json.get("description") {
            metadata.description = description.to_string();
        }
        if let Some(Value::String(author)) = json.get("author") {
            metadata.author = Some((
                author.to_string(),
                json.get("author_email")
                    .map(|x| x.to_string())
                    .map(escape_string)
                    .unwrap_or_else(String::new),
            ));
        }
        if let Some(Value::String(requires_python)) = json.get("requires_python") {
            metadata.requires_python = Some(requires_python.to_string());
        }
        if let Some(Value::String(license)) = json.get("license") {
            metadata.license = Some(license.to_string());
        }
        if let Some(Value::Array(reqs)) = json.get("install_requires") {
            reqs.iter()
                .filter_map(|x| Requirement::from_str(&x.to_string()).ok())
                .for_each(|x| {
                    requirements.insert(x.name.to_string(), x.to_string());
                });
        }
    }

    if let Some(paths) = requirements_files {
        for p in paths {
            import_requirements(&mut requirements, p)?;
        }
    }
    if let Some(paths) = dev_requirements_files {
        for p in paths {
            import_requirements(&mut dev_requirements, p)?;
        }
    }
    if !requirements.is_empty() {
        metadata.dependencies = Some(requirements.into_values().collect());
    }
    if !dev_requirements.is_empty() {
        metadata.dev_dependencies = Some(dev_requirements.into_values().collect());
    }

    Ok(metadata)
}

fn get_setup_py_json<T: AsRef<Path>>(path: T, python: T) -> Result<Value, Error> {
    let python = python.as_ref();
    let setup_py = path.as_ref();
    let temp_dir = tempdir()?;
    let dir = setup_py
        .parent()
        .context("could not establish setup.py parent dir")?;
    copy_dir(dir, temp_dir.path())?;

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

// TODO(cnpryer): A more robust parse + caveats
fn import_requirements<T: AsRef<Path>>(
    requirements: &mut HashMap<String, String>,
    path: T,
) -> Result<(), Error> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);
    for line in reader.lines() {
        if let Ok(req) = Requirement::from_str(&line?) {
            requirements
                .entry(req.name.to_string())
                .or_insert(req.to_string());
        }
    }
    Ok(())
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
            fs::create_dir_all(&dest)?;
        }
        for entry in fs::read_dir(working_path)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Some(filename) = path.file_name() {
                fs::copy(&path, dest.join(filename))?;
            }
        }
    }

    Ok(())
}
