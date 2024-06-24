use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Error};
use once_cell::sync::Lazy;
use pep440_rs::Operator;
use regex::Regex;
use toml_edit::DocumentMut;

use crate::platform::{get_app_dir, get_latest_cpython_version};
use crate::pyproject::{BuildSystem, SourceRef, SourceRefType};
use crate::sources::py::PythonVersionRequest;
use crate::utils::{toml, IoPathContext};

static CONFIG: Mutex<Option<Arc<Config>>> = Mutex::new(None);
static AUTHOR_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*(.*?)\s*<\s*(.*?)\s*>\s*$").unwrap());

pub fn load() -> Result<(), Error> {
    let cfg_path = get_app_dir().join("config.toml");
    let cfg = if cfg_path.is_file() {
        Config::from_path(&cfg_path)?
    } else {
        Config {
            doc: DocumentMut::new(),
            path: cfg_path,
        }
    };
    *CONFIG.lock().unwrap() = Some(Arc::new(cfg));
    Ok(())
}

#[derive(Clone)]
pub struct Config {
    doc: DocumentMut,
    path: PathBuf,
}

impl Config {
    /// Returns the current config
    pub fn current() -> Arc<Config> {
        CONFIG
            .lock()
            .unwrap()
            .as_ref()
            .expect("config not initialized")
            .clone()
    }

    /// Returns a clone of the internal doc.
    pub fn doc_mut(&mut self) -> &mut DocumentMut {
        &mut self.doc
    }

    /// Saves changes back.
    pub fn save(&self) -> Result<(), Error> {
        // try to make the parent folder if it does not exist.  ignore the error though.
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(&self.path, self.doc.to_string())
            .path_context(&self.path, "failed to save config")?;
        Ok(())
    }

    /// Returns the path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Loads a config from a path.
    pub fn from_path(path: &Path) -> Result<Config, Error> {
        let contents = fs::read_to_string(path).path_context(path, "failed to read config")?;
        Ok(Config {
            doc: contents
                .parse::<DocumentMut>()
                .path_context(path, "failed to parse config")?,
            path: path.to_path_buf(),
        })
    }

    /// Returns the default lower bound Python.
    pub fn default_requires_python(&self) -> String {
        match self
            .doc
            .get("default")
            .and_then(|x| x.get("requires-python"))
            .and_then(|x| x.as_str())
        {
            Some(ver) => {
                if ver.trim().parse::<pep440_rs::Version>().is_ok() {
                    format!(">= {}", ver)
                } else {
                    ver.to_string()
                }
            }
            None => ">= 3.8".into(),
        }
    }

    /// Returns the default python toolchain
    pub fn default_toolchain(&self) -> Result<PythonVersionRequest, Error> {
        match self
            .doc
            .get("default")
            .and_then(|x| x.get("toolchain"))
            .and_then(|x| x.as_str())
        {
            Some(ver) => ver.parse(),
            None => get_latest_cpython_version().map(Into::into),
        }
        .context("failed to get default toolchain")
    }

    /// Returns the default build system
    pub fn default_build_system(&self) -> Option<BuildSystem> {
        match self
            .doc
            .get("default")
            .and_then(|x| x.get("build-system"))
            .and_then(|x| x.as_str())
        {
            Some(build_system) => build_system.parse::<BuildSystem>().ok(),
            None => None,
        }
    }

    /// Returns the default license
    pub fn default_license(&self) -> Option<String> {
        self.doc
            .get("default")
            .and_then(|x| x.get("license"))
            .and_then(|x| x.as_str())
            .map(|x| x.to_string())
    }

    /// Returns the default author.
    pub fn default_author(&self) -> (Option<String>, Option<String>) {
        self.doc
            .get("default")
            .and_then(|x| x.get("author"))
            .and_then(|x| x.as_str())
            .map(|x| {
                if let Some(c) = AUTHOR_REGEX.captures(x) {
                    (Some(c[1].to_string()), Some(c[2].to_string()))
                } else {
                    (Some(x.to_string()), None)
                }
            })
            .unwrap_or_default()
    }

    /// Should dependencies added by default by pinned with ~= or ==
    pub fn default_dependency_operator(&self) -> Operator {
        self.doc
            .get("default")
            .and_then(|x| {
                x.get("dependency-operator")
                    // legacy typo key
                    .or_else(|| x.get("dependency_operator"))
            })
            .and_then(|x| x.as_str())
            .map_or(Operator::GreaterThanEqual, |x| match x {
                "==" => Operator::Equal,
                "~=" => Operator::TildeEqual,
                ">=" => Operator::GreaterThanEqual,
                _ => Operator::GreaterThanEqual,
            })
    }

    /// Allow rye shims to resolve globally installed Pythons.
    pub fn global_python(&self) -> bool {
        self.doc
            .get("behavior")
            .and_then(|x| x.get("global-python"))
            .and_then(|x| x.as_bool())
            .unwrap_or(false)
    }

    /// Pretend that all projects are rye managed.
    pub fn force_rye_managed(&self) -> bool {
        self.doc
            .get("behavior")
            .and_then(|x| {
                x.get("force-rye-managed")
                    // legacy typo key
                    .or_else(|| x.get("force_rye_managed"))
            })
            .and_then(|x| x.as_bool())
            .unwrap_or(false)
    }

    /// Mark the `.venv` to not sync to cloud storage
    pub fn venv_mark_sync_ignore(&self) -> bool {
        self.doc
            .get("behavior")
            .and_then(|x| x.get("venv-mark-sync-ignore"))
            .and_then(|x| x.as_bool())
            .unwrap_or(true)
    }

    /// Returns the HTTP proxy that should be used.
    pub fn http_proxy_url(&self) -> Option<String> {
        std::env::var("http_proxy").ok().or_else(|| {
            self.doc
                .get("proxy")
                .and_then(|x| x.get("http"))
                .and_then(|x| x.as_str())
                .map(|x| x.to_string())
        })
    }

    /// Returns the HTTPS proxy that should be used.
    pub fn https_proxy_url(&self) -> Option<String> {
        std::env::var("HTTPS_PROXY")
            .ok()
            .or_else(|| std::env::var("https_proxy").ok())
            .or_else(|| {
                self.doc
                    .get("proxy")
                    .and_then(|x| x.get("https"))
                    .and_then(|x| x.as_str())
                    .map(|x| x.to_string())
            })
    }

    /// Returns the list of default sources.
    pub fn sources(&self) -> Result<Vec<SourceRef>, Error> {
        let mut rv = Vec::new();
        let mut need_default = true;
        if let Some(sources) = self.doc.get("sources").map(|x| toml::iter_tables(x)) {
            for source in sources {
                let source = source.context("invalid value for source in config.toml")?;
                let source_ref = SourceRef::from_toml_table(source)
                    .context("invalid source definition in config.toml")?;
                if source_ref.name == "default" {
                    need_default = false;
                }
                rv.push(source_ref);
            }
        }

        if need_default {
            rv.push(SourceRef::from_url(
                "default".to_string(),
                "https://pypi.org/simple/".into(),
                SourceRefType::Index,
            ));
        }

        Ok(rv)
    }

    /// Enable autosync.
    pub fn autosync(&self) -> bool {
        self.doc
            .get("behavior")
            .and_then(|x| x.get("autosync"))
            .and_then(|x| x.as_bool())
            .unwrap_or_else(|| self.use_uv())
    }

    /// Indicates if uv should be used instead of pip-tools.
    pub fn use_uv(&self) -> bool {
        self.doc
            .get("behavior")
            .and_then(|x| x.get("use-uv"))
            .and_then(|x| x.as_bool())
            .unwrap_or(true)
    }

    /// Fetches python installations with build info if possible.
    ///
    /// This used to be the default behavior in Rye prior to 0.31.
    pub fn fetch_with_build_info(&self) -> bool {
        self.doc
            .get("behavior")
            .and_then(|x| x.get("fetch-with-build-info"))
            .and_then(|x| x.as_bool())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::{tempdir, TempDir};

    fn setup_config(contents: &str) -> (PathBuf, TempDir) {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("config.toml");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "{}", contents).unwrap();
        (file_path, dir) // Return the path and the TempDir to keep it alive
    }

    #[test]
    fn test_load_config() {
        let (cfg_path, _temp_dir) = setup_config("[default]\nrequires-python = '>= 3.6'");

        assert!(
            cfg_path.exists(),
            "Config file does not exist at {:?}",
            cfg_path
        );

        let load_result = Config::from_path(&cfg_path);
        assert!(
            load_result.is_ok(),
            "Failed to load config: {:?}",
            load_result.err()
        );

        let cfg = Config::from_path(&cfg_path).expect("Failed to load config");
        assert_eq!(cfg.default_requires_python(), ">= 3.6");
    }

    #[test]
    fn test_default_requires_python() {
        let (cfg_path, _temp_dir) = setup_config("");
        let cfg = Config::from_path(&cfg_path).expect("Failed to load config");
        assert_eq!(cfg.default_requires_python(), ">= 3.8");
    }

    #[test]
    fn test_default_toolchain() {
        let (cfg_path, _temp_dir) = setup_config("[default]\ntoolchain = '3.8'");
        let cfg = Config::from_path(&cfg_path).expect("Failed to load config");
        let toolchain = cfg.default_toolchain().unwrap();
        assert_eq!(toolchain.major, 3);
        assert_eq!(toolchain.minor.unwrap(), 8);
    }
    #[test]
    fn test_default_build_system() {
        let (cfg_path, _temp_dir) = setup_config("[default]\nbuild-system = 'setuptools'");
        let cfg = Config::from_path(&cfg_path).expect("Failed to load config");
        assert_eq!(cfg.default_build_system(), Some(BuildSystem::Setuptools));
    }

    #[test]
    fn test_default_license() {
        let (cfg_path, _temp_dir) = setup_config("[default]\nlicense = 'MIT'");
        let cfg = Config::from_path(&cfg_path).expect("Failed to load config");
        assert_eq!(cfg.default_license(), Some("MIT".to_string()));
    }

    #[test]
    fn test_default_author() {
        let (cfg_path, _temp_dir) = setup_config(
            r#"[default]
author = "John Doe <john@example.com>""#,
        );
        let cfg = Config::from_path(&cfg_path).expect("Failed to load config");
        let (name, email) = cfg.default_author();
        assert_eq!(name, Some("John Doe".to_string()));
        assert_eq!(email, Some("john@example.com".to_string()));
    }

    #[test]
    fn test_global_python() {
        let (cfg_path, _temp_dir) = setup_config("[behavior]\nglobal-python = true");
        let cfg = Config::from_path(&cfg_path).expect("Failed to load config");
        assert!(cfg.global_python());
    }

    #[test]
    fn test_force_rye_managed() {
        let (cfg_path, _temp_dir) = setup_config("[behavior]\nforce-rye-managed = true");
        let cfg = Config::from_path(&cfg_path).expect("Failed to load config");
        assert!(cfg.force_rye_managed());
    }

    #[test]
    fn test_venv_mark_sync_ignore() {
        let (cfg_path, _temp_dir) = setup_config("[behavior]\nvenv-mark-sync-ignore = false");
        let cfg = Config::from_path(&cfg_path).expect("Failed to load config");
        assert!(!cfg.venv_mark_sync_ignore());
    }

    #[test]
    fn test_http_proxy_url() {
        let (cfg_path, _temp_dir) = setup_config("[proxy]\nhttp = 'http://proxy.example.com'");
        let cfg = Config::from_path(&cfg_path).expect("Failed to load config");
        assert_eq!(
            cfg.http_proxy_url(),
            Some("http://proxy.example.com".to_string())
        );
    }

    #[test]
    fn test_https_proxy_url() {
        let (cfg_path, _temp_dir) = setup_config("[proxy]\nhttps = 'https://proxy.example.com'");
        let cfg = Config::from_path(&cfg_path).expect("Failed to load config");
        assert_eq!(
            cfg.https_proxy_url(),
            Some("https://proxy.example.com".to_string())
        );
    }

    #[test]
    fn test_sources_default_inclusion() {
        let (cfg_path, _temp_dir) = setup_config("");
        let cfg = Config::from_path(&cfg_path).expect("Failed to load config");
        let sources = cfg.sources().expect("Failed to get sources");
        assert!(sources
            .iter()
            .any(|src| src.name == "default" && src.url == "https://pypi.org/simple/"));
    }

    #[test]
    fn test_use_uv() {
        let (cfg_path, _temp_dir) = setup_config("[behavior]\nuse-uv = true");
        let cfg = Config::from_path(&cfg_path).expect("Failed to load config");
        // Assuming cfg!(windows) is false in this test environment
        assert!(cfg.use_uv());
    }
}
