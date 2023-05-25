use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::{env, fs};

use anyhow::{anyhow, bail, Context, Error};
use clap::{Parser, ValueEnum};
use console::style;
use license::License;
use minijinja::{context, Environment};
use pep440_rs::{Version, VersionSpecifier};
use serde::Serialize;

use crate::config::Config;
use crate::platform::{get_default_author, get_latest_cpython, get_python_version_from_pyenv_pin};
use crate::utils::is_inside_git_work_tree;

#[derive(ValueEnum, Copy, Clone, Serialize, Debug)]
#[value(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum BuildSystem {
    Hatchling,
    Setuptools,
    Filt,
    Pdm,
}

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
    /// Which build system should be used?
    #[arg(long, default_value = "hatchling")]
    build_system: BuildSystem,
    /// Which license should be used (SPDX identifier)?
    #[arg(long)]
    license: Option<String>,
    /// The name of the package.
    #[arg(long)]
    name: Option<String>,
}

/// The pyproject.toml template
///
/// This uses a template just to simplify the flexibility of emitting it.
const TOML_TEMPLATE: &str = r#"[project]
name = {{ name }}
version = {{ version }}
description = "Add a short description here"
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
        None => get_python_version_from_pyenv_pin()
            .map(|x| format!(">= {}.{}", x.major, x.minor))
            .unwrap_or_else(|| cfg.default_requires_python()),
    };
    let py = match cmd.py {
        Some(py) => py,
        None => {
            let version = get_python_version_from_pyenv_pin()
                .map(Ok)
                .unwrap_or_else(get_latest_cpython)?;
            format!("{}.{}.{}", version.major, version.minor, version.patch)
        }
    };
    if !VersionSpecifier::from_str(&requires_python)
        .map_err(|msg| anyhow!("invalid version specifier: {}", msg))?
        .contains(&Version::from_str(&py).map_err(|msg| anyhow!("invalid version: {}", msg))?)
    {
        eprintln!(
            "{} conflicted python version with project's requires-python, will auto fix it.",
            style("warning:").red()
        );
        requires_python = format!(">= {}", py.split('.').take(2).collect::<Vec<_>>().join("."));
    }

    let name = slug::slugify(
        cmd.name
            .unwrap_or_else(|| dir.file_name().unwrap().to_string_lossy().into_owned()),
    );
    let version = "0.1.0";
    let author = get_default_author();
    let license = if let Some(license) = cmd.license {
        if !license_file.is_file() {
            let license_obj: &dyn License = license
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
        };
        Some(license)
    } else {
        None
    };

    // write .python-version
    if !python_version_file.is_file() {
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
                name,
                license,
            },
        )?;
        fs::write(&readme, rv)?;
        true
    } else {
        false
    };

    let rv = env.render_named_str(
        "pyproject.json",
        TOML_TEMPLATE,
        context! {
            name,
            version,
            author,
            requires_python,
            license,
            with_readme,
            build_system => cmd.build_system,
        },
    )?;
    fs::write(&toml, rv).context("failed to write pyproject.toml")?;

    let src_dir = dir.join("src");
    if !src_dir.is_dir() {
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

    eprintln!(
        "{} Initialized project in {}",
        style("success:").green(),
        dir.display()
    );
    eprintln!("  Run `rye sync` to get started");

    Ok(())
}
