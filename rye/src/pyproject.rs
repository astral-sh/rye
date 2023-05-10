use core::fmt;
use std::borrow::Cow;
use std::collections::HashSet;
use std::env;
use std::env::consts::{ARCH, OS};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Error};
use globset::Glob;
use once_cell::sync::Lazy;
use pep440_rs::{Operator, VersionSpecifiers};
use pep508_rs::Requirement;
use regex::Regex;
use toml_edit::{Array, Document, Formatted, Item, Table, Value};

use crate::config::get_python_version_from_pyenv_pin;
use crate::consts::VENV_BIN;
use crate::sources::{get_download_url, PythonVersion, PythonVersionRequest};
use crate::sync::VenvMarker;
use crate::utils::{expand_env_vars, format_requirement, get_short_executable_name, is_executable};

static NORMALIZATION_SPLIT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[-_.]+").unwrap());

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DependencyKind<'a> {
    Normal,
    Dev,
    Excluded,
    Optional(Cow<'a, str>),
}

impl<'a> fmt::Display for DependencyKind<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DependencyKind::Normal => f.write_str("regular"),
            DependencyKind::Dev => f.write_str("dev"),
            DependencyKind::Excluded => f.write_str("excluded"),
            DependencyKind::Optional(ref sect) => write!(f, "optional ({})", sect),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DependencyRef {
    raw: String,
}

impl fmt::Display for DependencyRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl DependencyRef {
    /// Creates a new dependency ref from the given string.
    pub fn new(s: &str) -> DependencyRef {
        DependencyRef { raw: s.to_string() }
    }

    /// Expands and parses the dependency ref into a requirement.
    ///
    /// The function is invoked for every referenced variable.
    pub fn expand<F>(&self, f: F) -> Result<Requirement, Error>
    where
        F: for<'a> FnMut(&'a str) -> Option<String>,
    {
        Ok(expand_env_vars(&self.raw, f).parse()?)
    }
}

/// A reference to a script
#[derive(Clone, Debug)]
pub enum Script {
    /// A command alias
    Cmd(Vec<String>),
    /// External script reference
    External(PathBuf),
}

impl fmt::Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Script::Cmd(args) => {
                for (idx, arg) in args.iter().enumerate() {
                    if idx > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", shlex::quote(arg))?;
                }
                Ok(())
            }
            Script::External(ref script) => write!(f, "external: {}", script.display()),
        }
    }
}

#[derive(Debug)]
pub struct Workspace {
    root: PathBuf,
    doc: Document,
    members: Vec<String>,
}

impl Workspace {
    /// Loads a workspace from a pyproject.toml and path
    fn try_load_from_toml(doc: &Document, path: &Path) -> Option<Workspace> {
        doc.get("tool")
            .and_then(|x| x.get("rye"))
            .and_then(|x| x.get("workspace"))
            .and_then(|x| x.as_table_like())
            .map(|workspace| Workspace {
                root: path.to_path_buf(),
                doc: doc.clone(),
                members: workspace
                    .get("members")
                    .and_then(|x| x.as_array())
                    .map(|x| {
                        x.iter()
                            .filter_map(|item| item.as_str().map(|x| x.to_string()))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default(),
            })
    }

    /// Discovers a pyproject toml
    #[allow(unused)]
    pub fn discover_from_path(path: &Path) -> Option<Workspace> {
        let mut here = path;

        loop {
            let project_file = here.join("pyproject.toml");
            if project_file.is_file() {
                if let Ok(contents) = fs::read_to_string(&project_file) {
                    if let Ok(doc) = contents.parse::<Document>() {
                        if let Some(workspace) = Workspace::try_load_from_toml(&doc, here) {
                            return Some(workspace);
                        }
                    }
                }
            }

            here = match here.parent() {
                Some(parent) => parent,
                None => break,
            };
        }

        None
    }

    /// The path of the workspace
    pub fn path(&self) -> Cow<'_, Path> {
        Cow::Borrowed(&self.root)
    }

    /// Checks if a project is a member of the declared workspace.
    pub fn is_member(&self, path: &Path) -> bool {
        let canonicalized = self.root.join(path);
        if let Ok(relative) = path.strip_prefix(canonicalized) {
            if relative == Path::new("") || self.members.is_empty() {
                true
            } else {
                let path = relative.to_string_lossy();
                for pattern in &self.members {
                    if let Ok(glob) = Glob::new(pattern) {
                        if glob.compile_matcher().is_match(&*path) {
                            return true;
                        }
                    }
                }
                false
            }
        } else {
            false
        }
    }

    /// Iterates through all projects in the workspace.
    pub fn iter_projects<'a>(
        self: &'a Arc<Self>,
    ) -> impl Iterator<Item = Result<PyProject, Error>> + 'a {
        walkdir::WalkDir::new(&self.root)
            .into_iter()
            .filter_map(move |entry| match entry {
                Ok(entry) => {
                    if entry.file_type().is_file()
                        && entry.file_name() == OsStr::new("pyproject.toml")
                    {
                        let project =
                            match PyProject::load_with_workspace(entry.path(), self.clone()) {
                                Ok(Some(project)) => project,
                                Ok(None) => return None,
                                Err(err) => return Some(Err(err)),
                            };
                        if self.is_member(entry.path().parent().unwrap()) {
                            return Some(Ok(project));
                        }
                    }
                    None
                }
                Err(err) => Some(Err(err.into())),
            })
    }

    /// Looks up a single project.
    pub fn get_project<'a>(self: &'a Arc<Self>, p: &str) -> Result<Option<PyProject>, Error> {
        let normalized_name = normalize_package_name(p);
        for project in self.iter_projects() {
            let project = project?;
            if project.normalized_name()? == normalized_name {
                return Ok(Some(project));
            }
        }
        Ok(None)
    }

    /// Returns the virtualenv path of the workspace.
    pub fn venv_path(&self) -> Cow<'_, Path> {
        Cow::Owned(self.root.join(".venv"))
    }

    /// Returns the project's target python version.
    ///
    /// That is the Python version that appears as lower bound in the
    /// pyproject toml.
    pub fn target_python_version(&self) -> Option<PythonVersionRequest> {
        resolve_target_python_version(&self.doc, &self.venv_path())
    }

    /// Returns the project's intended venv python version.
    ///
    /// This is the python version that should be used for virtualenvs.
    pub fn venv_python_version(&self) -> Result<PythonVersion, Error> {
        resolve_intended_venv_python_version(&self.doc)
    }
}

/// Helps working with pyproject.toml files
#[derive(Debug)]
pub struct PyProject {
    root: PathBuf,
    workspace: Option<Arc<Workspace>>,
    doc: Document,
}

impl PyProject {
    /// Discovers and loads a pyproject toml.
    pub fn discover() -> Result<PyProject, Error> {
        let pyproject_toml = match find_project_root() {
            Some(root) => root.join("pyproject.toml"),
            None => bail!("did not find pyproject.toml"),
        };
        Self::load(&pyproject_toml)
    }

    /// Loads a pyproject toml.
    pub fn load(filename: &Path) -> Result<PyProject, Error> {
        let root = filename.parent().unwrap_or(Path::new("."));
        let doc = fs::read_to_string(filename)?
            .parse::<Document>()
            .with_context(|| {
                format!(
                    "failed to parse pyproject.toml from {}",
                    &filename.display()
                )
            })?;
        let mut workspace = Workspace::try_load_from_toml(&doc, root).map(Arc::new);

        if workspace.is_none() {
            workspace = Workspace::discover_from_path(root).map(Arc::new);
        }

        if let Some(ref workspace) = workspace {
            if !workspace.is_member(root) {
                bail!(
                    "project {} is not part of pyproject workspace {}",
                    filename.display(),
                    workspace.path().display()
                );
            }
        }

        Ok(PyProject {
            root: root.to_owned(),
            workspace,
            doc,
        })
    }

    /// Loads a pyproject toml with a given workspace.
    ///
    /// If the project is not a member of the workspace, `None` is returned.
    pub fn load_with_workspace(
        filename: &Path,
        workspace: Arc<Workspace>,
    ) -> Result<Option<PyProject>, Error> {
        let root = filename.parent().unwrap_or(Path::new("."));
        let doc = fs::read_to_string(filename)?
            .parse::<Document>()
            .with_context(|| {
                format!(
                    "failed to parse pyproject.toml from {} in context of workspace {}",
                    &filename.display(),
                    workspace.path().display(),
                )
            })?;

        if !workspace.is_member(root) {
            return Ok(None);
        }

        Ok(Some(PyProject {
            root: root.to_owned(),
            workspace: Some(workspace),
            doc,
        }))
    }

    /// Returns a reference to the workspace.
    ///
    /// If something isn't a workspace, it's not returned.
    pub fn workspace(&self) -> Option<&Arc<Workspace>> {
        self.workspace.as_ref()
    }

    /// Is this the root project of the workspace?
    pub fn is_workspace_root(&self) -> bool {
        match self.workspace {
            Some(ref workspace) => workspace.path() == self.root_path(),
            None => true,
        }
    }

    /// Returns the project root path
    pub fn root_path(&self) -> Cow<'_, Path> {
        Cow::Borrowed(&self.root)
    }

    /// Returns the project workspace path.
    pub fn workspace_path(&self) -> Cow<'_, Path> {
        self.workspace
            .as_ref()
            .map(|x| x.path())
            .unwrap_or_else(|| self.root_path())
    }

    /// Returns the path to the toml file.
    pub fn toml_path(&self) -> Cow<'_, Path> {
        Cow::Owned(self.root.join("pyproject.toml"))
    }

    /// Returns the location of the virtualenv.
    pub fn venv_path(&self) -> Cow<'_, Path> {
        match self.workspace() {
            Some(ws) => ws.venv_path(),
            None => self.root.join(".venv").into(),
        }
    }

    /// Returns the virtualenv bin path of the virtualenv.
    pub fn venv_bin_path(&self) -> Cow<'_, Path> {
        Cow::Owned(self.venv_path().join(VENV_BIN))
    }

    /// Returns the project's target python version
    pub fn target_python_version(&self) -> Option<PythonVersionRequest> {
        if let Some(workspace) = self.workspace() {
            workspace.target_python_version()
        } else {
            resolve_target_python_version(&self.doc, &self.venv_path())
        }
    }

    /// Returns the project's intended venv python version.
    ///
    /// This is the python version that should be used for virtualenvs.
    pub fn venv_python_version(&self) -> Result<PythonVersion, Error> {
        if let Some(workspace) = self.workspace() {
            workspace.venv_python_version()
        } else {
            resolve_intended_venv_python_version(&self.doc)
        }
    }

    /// Set the target Python version.
    pub fn set_target_python_version(&mut self, version: &PythonVersionRequest) {
        let mut marker = format!(">= {}", version.major);
        if let Some(minor) = version.minor {
            marker.push('.');
            marker.push_str(&minor.to_string());
        }

        let project = self
            .doc
            .entry("project")
            .or_insert(Item::Table(Table::new()));
        project["requires-python"] = Item::Value(Value::String(Formatted::new(marker)));
    }

    /// Returns the project name.
    pub fn name(&self) -> Option<&str> {
        self.doc
            .get("project")
            .and_then(|x| x.get("name"))
            .and_then(|x| x.as_str())
    }

    /// Returns the normalized name.
    pub fn normalized_name(&self) -> Result<String, Error> {
        self.name()
            .map(normalize_package_name)
            .ok_or_else(|| anyhow!("project from '{}' has no name", self.root_path().display()))
    }

    /// Looks up a script
    pub fn get_script_cmd(&self, key: &str) -> Option<Script> {
        let external = self.venv_bin_path().join(key);

        if is_executable(&external) && !is_unsafe_script(&external) {
            return Some(Script::External(external));
        }

        let value = self
            .doc
            .get("tool")
            .and_then(|x| x.get("rye"))
            .and_then(|x| x.get("scripts"))
            .and_then(|x| x.get(key))?;
        if let Some(cmd) = value.as_str() {
            shlex::split(cmd).map(Script::Cmd)
        } else {
            value.as_array().map(|cmd| {
                Script::Cmd(
                    cmd.iter()
                        .map(|x| {
                            x.as_str()
                                .map(|x| x.to_string())
                                .unwrap_or_else(|| x.to_string())
                        })
                        .collect(),
                )
            })
        }
    }

    /// Returns a list of known scripts.
    pub fn list_scripts(&self) -> HashSet<String> {
        let mut rv = match self
            .doc
            .get("tool")
            .and_then(|x| x.get("rye"))
            .and_then(|x| x.get("scripts"))
            .and_then(|x| x.as_table_like())
        {
            Some(tbl) => tbl.iter().map(|x| x.0.to_string()).collect(),
            None => HashSet::new(),
        };
        for entry in fs::read_dir(&self.venv_bin_path())
            .ok()
            .into_iter()
            .flatten()
            .flatten()
        {
            if is_executable(&entry.path()) && !is_unsafe_script(&entry.path()) {
                rv.insert(get_short_executable_name(&entry.path()));
            }
        }
        rv
    }

    /// Returns a set of all extras.
    pub fn extras(&self) -> HashSet<&str> {
        self.doc
            .get("project")
            .and_then(|x| x.get("optional-dependencies"))
            .and_then(|x| x.as_table_like())
            .map_or(None.into_iter(), |x| {
                Some(x.iter().map(|x| x.0)).into_iter()
            })
            .flatten()
            .collect()
    }

    /// Adds a dependency.
    pub fn add_dependency(
        &mut self,
        req: &Requirement,
        kind: &DependencyKind,
    ) -> Result<(), Error> {
        let dependencies = match kind {
            DependencyKind::Normal => &mut self.doc["project"]["dependencies"],
            DependencyKind::Dev => &mut self.doc["tool"]["rye"]["dev-dependencies"],
            DependencyKind::Excluded => &mut self.doc["tool"]["rye"]["excluded-dependencies"],
            DependencyKind::Optional(ref section) => {
                // add this as a proper non-inline table if it's missing
                let table = &mut self.doc["project"]["optional-dependencies"];
                if table.is_none() {
                    *table = Item::Table(Table::new());
                }
                &mut table[section as &str]
            }
        };
        if dependencies.is_none() {
            *dependencies = Item::Value(Value::Array(Array::new()));
        }
        set_dependency(
            dependencies
                .as_array_mut()
                .ok_or_else(|| anyhow!("dependencies in pyproject.toml are malformed"))?,
            req,
        );
        Ok(())
    }

    /// Removes a dependency
    pub fn remove_dependency(
        &mut self,
        req: &Requirement,
        kind: DependencyKind,
    ) -> Result<Option<Requirement>, Error> {
        let dependencies = match kind {
            DependencyKind::Normal => &mut self.doc["project"]["dependencies"],
            DependencyKind::Dev => &mut self.doc["tool"]["rye"]["dev-dependencies"],
            DependencyKind::Excluded => &mut self.doc["tool"]["rye"]["excluded-dependencies"],
            DependencyKind::Optional(ref section) => {
                &mut self.doc["project"]["optional-dependencies"][section as &str]
            }
        };
        if !dependencies.is_none() {
            Ok(remove_dependency(
                dependencies
                    .as_array_mut()
                    .ok_or_else(|| anyhow!("dependencies in pyproject.toml are malformed"))?,
                req,
            ))
        } else {
            Ok(None)
        }
    }

    /// Iterates over all dependencies.
    pub fn iter_dependencies(
        &self,
        kind: DependencyKind,
    ) -> impl Iterator<Item = DependencyRef> + '_ {
        let sec = match kind {
            DependencyKind::Normal => self.doc.get("project").and_then(|x| x.get("dependencies")),
            DependencyKind::Dev => self
                .doc
                .get("tool")
                .and_then(|x| x.get("rye"))
                .and_then(|x| x.get("dev-dependencies")),
            DependencyKind::Excluded => self
                .doc
                .get("tool")
                .and_then(|x| x.get("rye"))
                .and_then(|x| x.get("excluded-dependencies")),
            DependencyKind::Optional(ref section) => self
                .doc
                .get("project")
                .and_then(|x| x.get("optional-dependencies"))
                .and_then(|x| x.get(section as &str)),
        };
        sec.and_then(|x| x.as_array())
            .into_iter()
            .flatten()
            .filter_map(|x| x.as_str())
            .map(DependencyRef::new)
    }

    /// Save back changes
    pub fn save(&self) -> Result<(), Error> {
        fs::write(self.toml_path(), self.doc.to_string()).with_context(|| {
            format!("unable to write changes to {}", self.toml_path().display())
        })?;
        Ok(())
    }
}

pub fn normalize_package_name(x: &str) -> String {
    NORMALIZATION_SPLIT_RE
        .split(x)
        .fold(String::new(), |mut acc, item| {
            if !acc.is_empty() {
                acc.push('-');
            }
            acc.push_str(&item.to_ascii_lowercase());
            acc
        })
}

fn set_dependency(deps: &mut Array, req: &Requirement) {
    let mut to_replace = None;
    for (idx, dep) in deps.iter().enumerate() {
        if let Some(dep) = dep.as_str() {
            if let Ok(dep_req) = Requirement::from_str(dep) {
                if dep_req.name.eq_ignore_ascii_case(&req.name) {
                    to_replace = Some(idx);
                    break;
                }
            }
        }
    }

    let formatted = format_requirement(req).to_string();
    if let Some(idx) = to_replace {
        deps.replace(idx, formatted);
    } else {
        deps.push(formatted);
    }
}

fn remove_dependency(deps: &mut Array, req: &Requirement) -> Option<Requirement> {
    let mut to_remove = None;
    for (idx, dep) in deps.iter().enumerate() {
        if let Some(dep) = dep.as_str() {
            if let Ok(dep_req) = Requirement::from_str(dep) {
                if dep_req.name.eq_ignore_ascii_case(&req.name) {
                    to_remove = Some(idx);
                    break;
                }
            }
        }
    }

    if let Some(idx) = to_remove {
        deps.remove(idx)
            .as_str()
            .and_then(|x| Requirement::from_str(x).ok())
    } else {
        None
    }
}

pub fn get_current_venv_python_version(venv_path: &Path) -> Option<PythonVersion> {
    let marker_file = venv_path.join("rye-venv.json");
    let contents = fs::read(marker_file).ok()?;
    let marker: VenvMarker = serde_json::from_slice(&contents).ok()?;
    Some(marker.python)
}

fn resolve_target_python_version(doc: &Document, venv_path: &Path) -> Option<PythonVersionRequest> {
    resolve_lower_bound_python_version(doc)
        .or_else(|| get_current_venv_python_version(venv_path).map(Into::into))
        .or_else(|| get_python_version_from_pyenv_pin().map(Into::into))
}

fn resolve_intended_venv_python_version(doc: &Document) -> Result<PythonVersion, Error> {
    if let Some(ver) = get_python_version_from_pyenv_pin() {
        return Ok(ver);
    }
    let requested_version = resolve_lower_bound_python_version(doc).ok_or_else(|| {
        anyhow!(
            "could not determine a target python version.  Define requires-python in \
                 pyproject.toml or use a .python-version file"
        )
    })?;

    if let Ok(ver) = PythonVersion::try_from(requested_version.clone()) {
        return Ok(ver);
    }

    if let Some((latest, _)) = get_download_url(&requested_version, OS, ARCH) {
        Ok(latest)
    } else {
        Err(anyhow!(
            "Unable to determine target virtualenv python version"
        ))
    }
}

fn resolve_lower_bound_python_version(doc: &Document) -> Option<PythonVersionRequest> {
    doc.get("project")
        .and_then(|x| x.get("requires-python"))
        .and_then(|x| x.as_str())
        .and_then(|s| s.parse::<VersionSpecifiers>().ok())
        .and_then(|versions| {
            versions
                .iter()
                .filter(|x| {
                    matches!(
                        x.operator(),
                        Operator::Equal
                            | Operator::EqualStar
                            | Operator::GreaterThanEqual
                            | Operator::GreaterThan
                    )
                })
                .map(|x| {
                    let mut rv = PythonVersionRequest::from(x.version().clone());
                    // this is pretty shitty, but probably good enough
                    if matches!(x.operator(), Operator::GreaterThan) {
                        if let Some(ref mut patch) = rv.patch {
                            *patch += 1;
                        } else if let Some(ref mut minor) = rv.minor {
                            *minor += 1;
                        }
                    }
                    rv
                })
                .min()
        })
}

pub fn find_project_root() -> Option<PathBuf> {
    let mut here = env::current_dir().ok()?;

    loop {
        let project_file = here.join("pyproject.toml");
        if project_file.is_file() {
            return Some(here.to_path_buf());
        }

        if !here.pop() {
            break;
        }
    }

    None
}

fn is_unsafe_script(path: &Path) -> bool {
    #[cfg(windows)]
    {
        let stem = path.file_stem();
        stem == Some(OsStr::new("activate")) || stem == Some(OsStr::new("deactivate"))
    }
    #[cfg(unix)]
    {
        let _ = path;
        false
    }
}
