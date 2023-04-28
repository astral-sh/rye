use std::path::PathBuf;
use std::{env, fs};

use anyhow::{bail, Context, Error};
use clap::{Parser, ValueEnum};
use console::style;
use minijinja::{context, Environment};
use serde::Serialize;

use crate::config::{get_default_author, load_python_version};

#[derive(ValueEnum, Copy, Clone, Serialize, Debug)]
#[value(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum BuildSystem {
    Hatchling,
    Setuptools,
    Filt,
}

/// Creates a new python project.
#[derive(Parser, Debug)]
pub struct Args {
    /// Where to place the project (defaults to current path)
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Which interpreter version should be used?
    #[arg(short, long)]
    py: Option<String>,
    /// Which build system should be used?
    #[arg(long, default_value = "hatchling")]
    build_system: BuildSystem,
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
readme = "README.md"
requires-python = {{ requires_python }}
license = { text = {{ license }} }

[build-system]
{%- if build_system == "hatchling" %}
requires = ["hatchling"]
build-backend = "hatchling.build"
{%- elif build_system == "setuptools" %}
requires = ["setuptools>=61.0"]
build-backend = "setuptools.build_meta"
{%- elif build_system == "filt" %}
requires = ["flit_core>=3.4"]
build-backend = "filt_core.buildapi"
{%- endif %}

[tool.rye]
managed = true

"#;

const README_TEMPLATE: &str = r#"# {{ name }}

Describe your project here.

* License: {{ license }}

"#;

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
    let env = Environment::new();
    let dir = env::current_dir()?.join(cmd.path);
    let toml = dir.join("pyproject.toml");
    let readme = dir.join("README.md");
    let gitignore = dir.join(".gitignore");

    if toml.is_file() {
        bail!("pyproject.toml already exists");
    }

    // fail silently if it already exists or cannot be created.
    fs::create_dir_all(&dir).ok();

    // Write pyproject.toml
    let py = match cmd.py {
        Some(py) => py,
        None => load_python_version()
            .map(|x| format!("{}.{}", x.major, x.minor))
            .unwrap_or_else(|| "3.8".into()),
    };
    let name = slug::slugify(dir.file_name().unwrap().to_string_lossy());
    let version = "0.1.0";
    let requires_python = format!(">= {}", py);
    let author = get_default_author();
    let license = "MIT";

    let rv = env.render_named_str(
        "pyproject.json",
        TOML_TEMPLATE,
        context! {
            name,
            version,
            author,
            requires_python,
            license,
            build_system => cmd.build_system,
        },
    )?;
    fs::write(&toml, rv).context("failed to write pyproject.toml")?;

    // create a readme if one is missing
    if !readme.is_file() {
        let rv = env.render_named_str(
            "README.txt",
            README_TEMPLATE,
            context! {
                name,
                license,
            },
        )?;
        fs::write(&readme, rv)?;
    }

    // create a .gitignore if one is missing
    if !gitignore.is_file() {
        let rv = env.render_named_str("gitignore.txt", GITIGNORE_TEMPLATE, ())?;
        fs::write(&gitignore, rv).context("failed to write .gitignore")?;
    }

    eprintln!(
        "{} Initialized project in {}",
        style("success:").green(),
        dir.display()
    );

    Ok(())
}
