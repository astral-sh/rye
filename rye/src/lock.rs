use std::collections::HashSet;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use anyhow::{bail, Context, Error};
use tempfile::NamedTempFile;

use crate::bootstrap::ensure_self_venv;
use crate::pyproject::{normalize_package_name, DependencyKind, PyProject, Workspace};
use crate::utils::CommandOutput;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LockMode {
    Production,
    Dev,
}

impl fmt::Display for LockMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LockMode::Production => "production",
                LockMode::Dev => "dev",
            }
        )
    }
}

/// Controls how locking should work.
#[derive(Debug, Clone, Default)]
pub struct LockOptions {
    /// Instruct all packages to update.
    pub update_all: bool,
    /// Update specific packages.
    pub update: Vec<String>,
    /// Pick pre-release versions.
    pub pre: bool,
}

fn get_pip_compile(output: CommandOutput) -> Result<PathBuf, Error> {
    let mut pip_compile = ensure_self_venv(output)?;
    pip_compile.push("bin");
    pip_compile.push("pip-compile");
    Ok(pip_compile)
}

/// Creates lockfiles for all projects in the workspace.
pub fn update_workspace_lockfile(
    workspace: &Arc<Workspace>,
    lock_mode: LockMode,
    lockfile: &Path,
    output: CommandOutput,
    lock_options: &LockOptions,
) -> Result<(), Error> {
    if output != CommandOutput::Quiet {
        eprintln!("Generating {} lockfile: {}", lock_mode, lockfile.display());
    }

    let mut req_file = NamedTempFile::new()?;
    let mut local_req_file = NamedTempFile::new()?;

    let mut local_projects = HashSet::new();
    let mut projects = Vec::new();
    for pyproject_result in workspace.iter_projects() {
        let pyproject = pyproject_result?;
        writeln!(local_req_file, "-e {}", pyproject.root_path().display())?;
        if let Some(name) = pyproject.normalized_name() {
            local_projects.insert(name);
        }
        projects.push(pyproject);
    }

    for pyproject in projects {
        for dep in pyproject.iter_dependencies(DependencyKind::Normal) {
            if !local_projects.contains(&normalize_package_name(&dep.name)) {
                writeln!(req_file, "{}", dep)?;
            }
        }
        if lock_mode == LockMode::Dev {
            for dep in pyproject.iter_dependencies(DependencyKind::Dev) {
                if !local_projects.contains(&normalize_package_name(&dep.name)) {
                    writeln!(req_file, "{}", dep)?;
                }
            }
        }
    }

    generate_lockfile(output, req_file.path(), lockfile, lock_options, &[])?;
    generate_lockfile(
        output,
        local_req_file.path(),
        lockfile,
        lock_options,
        &["--pip-args=--no-deps"],
    )?;

    Ok(())
}

/// Updates the lockfile of the current project.
pub fn update_single_project_lockfile(
    pyproject: &PyProject,
    lock_mode: LockMode,
    lockfile: &Path,
    output: CommandOutput,
    lock_options: &LockOptions,
) -> Result<(), Error> {
    if output != CommandOutput::Quiet {
        eprintln!("Generating {} lockfile: {}", lock_mode, lockfile.display());
    }

    let mut req_file = NamedTempFile::new()?;
    writeln!(req_file, "-e {}", pyproject.root_path().display())?;
    for dep in pyproject.iter_dependencies(DependencyKind::Normal) {
        writeln!(req_file, "{}", dep)?;
    }
    if lock_mode == LockMode::Dev {
        for dep in pyproject.iter_dependencies(DependencyKind::Dev) {
            writeln!(req_file, "{}", dep)?;
        }
    }

    generate_lockfile(output, req_file.path(), lockfile, lock_options, &[])?;

    Ok(())
}

fn generate_lockfile(
    output: CommandOutput,
    requirements_file_in: &Path,
    lockfile: &Path,
    lock_options: &LockOptions,
    extra_args: &[&str],
) -> Result<(), Error> {
    let pip_compile_path = get_pip_compile(output)?;
    let mut cmd = Command::new(pip_compile_path);
    cmd.arg("--resolver=backtracking")
        .arg("--no-annotate")
        .arg("--strip-extras")
        .arg("--allow-unsafe")
        .arg("--no-header")
        .arg("-o")
        .arg(lockfile)
        .arg(requirements_file_in)
        .env("PYTHONWARNINGS", "ignore");
    if output == CommandOutput::Verbose {
        cmd.arg("--verbose");
    } else {
        cmd.arg("-q");
    }
    for pkg in &lock_options.update {
        cmd.arg("--upgrade-package");
        cmd.arg(pkg);
    }
    if lock_options.update_all {
        cmd.arg("--upgrade");
    }
    if lock_options.pre {
        cmd.arg("--pre");
    }
    cmd.args(extra_args);
    let status = cmd.status().context("unable to run pip-compile")?;
    if !status.success() {
        bail!("failed to generate lockfile");
    };
    Ok(())
}
