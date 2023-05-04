use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::{env, fs};

use anyhow::{bail, Context, Error};
use clap::{Parser, ValueEnum};
use console::style;
use license::License;
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
    /// Do not create a readme.
    #[arg(long)]
    no_readme: bool,
    /// Which build system should be used?
    #[arg(long, default_value = "hatchling")]
    build_system: BuildSystem,
    #[arg(long)]
    license: Option<String>,
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

/// The template for the readme file.
const README_TEMPLATE: &str = r#"# {{ name }}

Describe your project here.

* License: {{ license }}

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
    let env = Environment::new();
    let dir = env::current_dir()?.join(cmd.path);
    let toml = dir.join("pyproject.toml");
    let readme = dir.join("README.md");
    let license_file = dir.join("LICENSE.txt");

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
    let license = match cmd.license {
        Some(license) => {
            if !license_file.is_file() {
                let license_obj: &'static dyn License = <&'static dyn License>::from_str(&license)
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
            license
        }
        None => "MIT".to_string(),
    };

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
    if !dir.join(".git").is_dir()
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
