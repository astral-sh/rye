use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use insta_cmd::get_cargo_bin;
use tempfile::TempDir;

// Exclude any packages uploaded after this date.
pub static EXCLUDE_NEWER: &str = "2023-11-18T12:00:00Z";

#[allow(unused)]
pub const INSTA_FILTERS: &[(&str, &str)] = &[
    // general temp folders
    (
        r"(\b[A-Z]:)?[\\/].*?[\\/]\.rye-tests---[^\\/]+[\\/]",
        "[TEMP_PATH]/",
    ),
    // home
    (r"(\b[A-Z]:)?[\\/].*?[\\/]rye-test-home", "[RYE_HOME]"),
    // macos temp folder
    (r"/var/folders/\S+?/T/\S+", "[TEMP_FILE]"),
    // linux temp folders
    (r"/tmp/\.tmp\S+", "[TEMP_FILE]"),
    // windows temp folders
    (r"\b[A-Z]:\\.*\\Local\\Temp\\\S+", "[TEMP_FILE]"),
    (r" in (\d+\.)?\d+(ms|s)\b", " in [EXECUTION_TIME]"),
    (r"\\\\?([\w\d.])", "/$1"),
    (r"rye.exe", "rye"),
];

fn marked_tempdir() -> TempDir {
    TempDir::with_prefix(".rye-tests---").unwrap()
}

fn bootstrap_test_rye() -> PathBuf {
    let home = get_bin().parent().unwrap().join("rye-test-home");
    fs::create_dir_all(&home).ok();
    let lock_path = home.join("lock");
    let mut lock = fslock::LockFile::open(&lock_path).unwrap();
    lock.lock().unwrap();

    // write config
    let config_file = home.join("config.toml");
    if !config_file.is_file() {
        fs::write(
            home.join("config.toml"),
            r#"
[behavior]
use-uv = true

[default]
toolchain = "cpython@3.12.1"
"#,
        )
        .unwrap();
    }

    // fetch the most important interpreters
    for version in ["cpython@3.8.17", "cpython@3.11.7", "cpython@3.12.1"] {
        if home.join("py").join(version).is_dir() {
            continue;
        }
        let status = Command::new(get_bin())
            .env("RYE_HOME", &home)
            .arg("fetch")
            .arg(version)
            .status()
            .unwrap();
        assert!(status.success());
    }

    // make a dummy project to bootstrap it
    if !home.join("self").is_dir() {
        let t = marked_tempdir();
        Command::new(get_bin())
            .env("RYE_HOME", &home)
            .current_dir(t.path())
            .arg("init")
            .arg("--name=test-project")
            .status()
            .unwrap();
        Command::new(get_bin())
            .env("RYE_HOME", &home)
            .current_dir(t.path())
            .arg("sync")
            .status()
            .unwrap();
    }

    lock.unlock().unwrap();

    home
}

pub fn get_bin() -> PathBuf {
    get_cargo_bin("rye")
}

pub struct Space {
    #[allow(unused)]
    tempdir: TempDir,
    rye_home: PathBuf,
    project_dir: PathBuf,
}

impl Space {
    pub fn new() -> Space {
        let tempdir = marked_tempdir();
        let project_dir = tempdir.path().join("project");
        let rye_home = bootstrap_test_rye();
        fs::create_dir_all(&project_dir).unwrap();
        Space {
            tempdir,
            project_dir,
            rye_home,
        }
    }

    pub fn cmd<S>(&self, cmd: S) -> Command
    where
        S: AsRef<OsStr>,
    {
        let mut rv = Command::new(cmd);
        rv.env("RYE_HOME", self.rye_home().as_os_str());
        rv.env("UV_CACHE_DIR", self.tempdir.path().join("uv-cache"));
        rv.env("__RYE_UV_EXCLUDE_NEWER", EXCLUDE_NEWER);
        rv.current_dir(self.project_path());
        rv
    }

    pub fn rye_cmd(&self) -> Command {
        self.cmd(get_bin())
    }

    #[allow(unused)]
    pub fn edit_toml<P: AsRef<Path>, R, F: FnOnce(&mut toml_edit::Document) -> R>(
        &self,
        path: P,
        f: F,
    ) -> R {
        let p = self.project_path().join(path.as_ref());
        let mut doc = if p.is_file() {
            std::fs::read_to_string(&p).unwrap().parse().unwrap()
        } else {
            toml_edit::Document::default()
        };
        let rv = f(&mut doc);
        fs::create_dir_all(p.parent().unwrap()).ok();
        fs::write(p, doc.to_string()).unwrap();
        rv
    }

    #[allow(unused)]
    pub fn read_toml<P: AsRef<Path>>(&self, path: P) -> toml_edit::Document {
        let p = self.project_path().join(path.as_ref());
        std::fs::read_to_string(&p).unwrap().parse().unwrap()
    }

    #[allow(unused)]
    pub fn write<P: AsRef<Path>, B: AsRef<[u8]>>(&self, path: P, contents: B) {
        let p = self.project_path().join(path.as_ref());
        fs::create_dir_all(p.parent().unwrap()).ok();
        fs::write(p, contents).unwrap();
    }

    #[allow(unused)]
    pub fn read_string<P: AsRef<Path>>(&self, path: P) -> String {
        let p = self.project_path().join(path.as_ref());
        fs::read_to_string(p).unwrap()
    }

    #[allow(unused)]
    pub fn init(&self, name: &str) {
        let status = self
            .cmd(get_bin())
            .arg("init")
            .arg("--name")
            .arg(name)
            .arg("-q")
            .current_dir(self.project_path())
            .status()
            .unwrap();
        assert!(status.success());
    }

    pub fn rye_home(&self) -> &Path {
        &self.rye_home
    }

    pub fn project_path(&self) -> &Path {
        &self.project_dir
    }

    #[allow(unused)]
    pub fn lock_rye_home(&self) -> fslock::LockFile {
        let mut lock = fslock::LockFile::open(&self.rye_home().join("lock")).unwrap();
        lock.lock().unwrap();
        lock
    }
}

#[allow(unused_macros)]
macro_rules! rye_cmd_snapshot {
    ($cmd:expr, @$snapshot:literal) => {{
        let mut settings = insta::Settings::clone_current();
        for (matcher, replacement) in $crate::common::INSTA_FILTERS {
            settings.add_filter(matcher, *replacement);
        }
        let _guard = settings.bind_to_scope();
        insta_cmd::assert_cmd_snapshot!($cmd, @$snapshot);
    }};
}

#[allow(unused_imports)]
pub(crate) use rye_cmd_snapshot;
