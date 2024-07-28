use crate::utils::IoPathContext;
use anyhow::{anyhow, Error};
use clap::ValueEnum;
use minijinja::Environment;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::str::FromStr;

/// Template for fresh gitignore files.
const GITIGNORE_TEMPLATE: &str = include_str!("templates/gitignore.j2");

// Template for initial hgignore file
const HGIGNORE_TEMPLATE: &str = include_str!("templates/hgignore.j2");

#[derive(ValueEnum, Copy, Clone, Serialize, Debug, PartialEq)]
#[value(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ProjectVCS {
    None,
    Git,
    Mercurial,
}

impl FromStr for ProjectVCS {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(ProjectVCS::None),
            "git" => Ok(ProjectVCS::Git),
            "mercurial" => Ok(ProjectVCS::Mercurial),
            _ => Err(anyhow!("unknown VCS")),
        }
    }
}

// Implement this trait for each VCS type and add mapping to ProjectVCS impl methods
trait VCSInfo {
    // Used to check whether a dir is already a VCS working tree. True if yes.
    fn inside_work_tree(dir: &Path) -> bool;
    // Used to init a new VCS repository in a dir. True if successful.
    fn init_dir(dir: &Path) -> bool;
    // Used to get author info from VCS metadata, for use in project metadata. Return (name?, email?)
    fn get_author(
        dir: &Path,
        or_defaults: (Option<String>, Option<String>),
    ) -> (Option<String>, Option<String>);
    // Run after init_dir to render any VCS-specific templates.
    fn render_templates<S: Serialize>(
        dir: &Path,
        env: &Environment,
        context: S,
    ) -> Result<(), Error>;
}

struct Git;
impl VCSInfo for Git {
    fn inside_work_tree(dir: &Path) -> bool {
        command_silent_as_bool(
            Command::new("git")
                .arg("rev-parse")
                .arg("--is-inside-work-tree")
                .current_dir(dir),
        )
    }

    fn init_dir(dir: &Path) -> bool {
        command_silent_as_bool(Command::new("git").arg("init").current_dir(dir))
    }

    fn get_author(
        dir: &Path,
        or_defaults: (Option<String>, Option<String>),
    ) -> (Option<String>, Option<String>) {
        let (default_name, default_email) = or_defaults;
        let mut name: Option<String> = None;
        let mut email: Option<String> = None;
        if let Ok(rv) = Command::new("git")
            .current_dir(dir)
            .arg("config")
            .arg("--get-regexp")
            .arg("^user.(name|email)$")
            .stdout(Stdio::piped())
            .output()
        {
            let command_output = std::str::from_utf8(&rv.stdout);
            match command_output {
                Err(_) => {}
                Ok(output) => {
                    for line in output.lines() {
                        match line.split_once(' ') {
                            Some((_, "")) => {}

                            Some(("user.email", value)) => {
                                email = Some(value.to_string());
                            }
                            Some(("user.name", value)) => {
                                name = Some(value.to_string());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        (name.or(default_name), email.or(default_email))
    }

    fn render_templates<S: Serialize>(
        dir: &Path,
        env: &Environment,
        context: S,
    ) -> Result<(), Error> {
        render_ignore_file(dir, env, context, ".gitignore", GITIGNORE_TEMPLATE)
    }
}

struct Mercurial;

impl VCSInfo for Mercurial {
    fn inside_work_tree(dir: &Path) -> bool {
        command_silent_as_bool(Command::new("hg").arg("root").current_dir(dir))
    }

    fn init_dir(dir: &Path) -> bool {
        command_silent_as_bool(Command::new("hg").arg("init").current_dir(dir))
    }

    fn get_author(
        dir: &Path,
        or_defaults: (Option<String>, Option<String>),
    ) -> (Option<String>, Option<String>) {
        let (default_name, default_email) = or_defaults;
        let mut name: Option<String> = None;
        let mut email: Option<String> = None;
        if let Ok(rv) = Command::new("hg")
            .current_dir(dir)
            .arg("config")
            .arg("get")
            .arg("ui.username")
            .arg("ui.email")
            .stdout(Stdio::piped())
            .output()
        {
            let command_output = std::str::from_utf8(&rv.stdout);
            match command_output {
                Err(_) => {}
                Ok(output) => {
                    for line in output.lines() {
                        match line.split_once('=') {
                            Some((_, "")) => {}
                            Some(("ui.email", value)) => {
                                email = Some(value.to_string());
                            }
                            Some(("ui.username", value)) => {
                                name = Some(value.to_string());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        (name.or(default_name), email.or(default_email))
    }

    fn render_templates<S: Serialize>(
        dir: &Path,
        env: &Environment,
        context: S,
    ) -> Result<(), Error> {
        render_ignore_file(dir, env, context, ".hgignore", HGIGNORE_TEMPLATE)
    }
}

impl ProjectVCS {
    // Is this dir inside a VCS working dir of this type?
    pub fn inside_work_tree(&self, dir: &Path) -> bool {
        match self {
            ProjectVCS::None => false,
            ProjectVCS::Mercurial => Mercurial::inside_work_tree(dir),
            ProjectVCS::Git => Git::inside_work_tree(dir),
        }
    }

    // Initialize dir as self type VCS repository
    pub fn init_dir(&self, dir: &Path) -> bool {
        match self {
            ProjectVCS::None => true,
            ProjectVCS::Git => Git::init_dir(dir),
            ProjectVCS::Mercurial => Mercurial::init_dir(dir),
        }
    }

    // Returns author in metadata form: (name, email) tuple from vcs, if it exists.
    pub fn get_author(
        &self,
        dir: &Path,
        or_defaults: (Option<String>, Option<String>),
    ) -> (Option<String>, Option<String>) {
        match self {
            ProjectVCS::None => or_defaults,
            ProjectVCS::Git => Git::get_author(dir, or_defaults),
            ProjectVCS::Mercurial => Mercurial::get_author(dir, or_defaults),
        }
    }

    // Render the support templates for this VCS to given dir
    pub fn render_templates<S: Serialize>(
        &self,
        dir: &Path,
        env: &Environment,
        context: S,
    ) -> Result<(), Error> {
        match self {
            ProjectVCS::None => Ok(()),
            ProjectVCS::Git => Git::render_templates(dir, env, context),
            ProjectVCS::Mercurial => Mercurial::render_templates(dir, env, context),
        }
    }
}

// maybe util
fn command_silent_as_bool(cmd: &mut Command) -> bool {
    cmd.stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn render_ignore_file<S: Serialize>(
    dir: &Path,
    env: &Environment,
    context: S,
    ignore_filename: &str,
    ignore_template: &str,
) -> Result<(), Error> {
    let vcs_ignore_path = dir.join(ignore_filename);
    if !vcs_ignore_path.is_file() {
        let rv = env.render_str(ignore_template, context);
        match rv {
            Err(e) => {
                return Err(anyhow!("failed to render ignore file template: {}", e));
            }
            Ok(rv) => {
                fs::write(&vcs_ignore_path, rv)
                    .path_context(&vcs_ignore_path, "failed to write {vcs_ignore_path}")?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod test_mercurial {
    use super::Mercurial;
    use super::VCSInfo;
    use std::path::PathBuf;
    use std::{env, fs};
    use tempfile::TempDir;

    fn hg_init_temp() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let hg_init_result = Mercurial::init_dir(&PathBuf::from(temp_dir.path()));
        assert!(hg_init_result);
        let hg_dir = temp_dir.path().join(".hg");
        assert!(hg_dir.exists());

        return temp_dir;
    }

    #[test]
    fn test_is_inside_hg_work_tree_true() {
        let temp_dir = hg_init_temp();
        assert!(Mercurial::inside_work_tree(&PathBuf::from(
            temp_dir.as_ref()
        )));
        temp_dir.close().unwrap();
    }

    #[test]
    fn test_is_inside_hg_work_tree_false() {
        let temp_dir = TempDir::new().unwrap();
        assert!(!Mercurial::inside_work_tree(&PathBuf::from(
            temp_dir.as_ref()
        )));
        temp_dir.close().unwrap();
    }

    #[test]
    fn test_hg_get_author_defaults() {
        // case 1: no author set, defaults set

        // disable normal mercurial config search path
        env::set_var("HGRCPATH", "");
        let temp_dir = hg_init_temp();
        let (name, email) = Mercurial::get_author(
            &PathBuf::from(temp_dir.as_ref()),
            (
                Some("defaultname".to_string()),
                Some("defaultemail".to_string()),
            ),
        );
        assert_eq!(name, Some("defaultname".to_string()));
        assert_eq!(email, Some("defaultemail".to_string()));
    }

    #[test]
    fn test_hg_get_author_hgrc() {
        // case 2: author set, defaults set

        let temp_dir = hg_init_temp();
        let hg_user = "hg_username";
        let hg_email = "hg_email";
        let hgrc_path = temp_dir.path().join(".hg").join("hgrc");

        fs::write(
            &hgrc_path,
            format!("[ui]\nusername = {}\nemail = {}", hg_user, hg_email).as_str(),
        )
        .unwrap();
        let (name, email) = Mercurial::get_author(
            &PathBuf::from(temp_dir.as_ref()),
            (
                Some("defaultname".to_string()),
                Some("defaultemail".to_string()),
            ),
        );
        assert_eq!(name, Some(hg_user.to_string()));
        assert_eq!(email, Some(hg_email.to_string()));
    }
}

#[cfg(test)]
mod test_git {
    use super::{Git, VCSInfo};
    use std::path::PathBuf;
    use std::{env, fs};
    use tempfile::TempDir;

    fn git_init_temp() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let hg_init_result = Git::init_dir(&PathBuf::from(temp_dir.path()));
        assert!(hg_init_result);
        let hg_dir = temp_dir.path().join(".git");
        assert!(hg_dir.exists());

        return temp_dir;
    }

    #[test]
    fn test_git_is_inside_work_tree_true() {
        let temp_dir = git_init_temp();
        assert!(Git::inside_work_tree(&PathBuf::from(temp_dir.as_ref())));
        temp_dir.close().unwrap();
    }

    #[test]
    fn test_git_is_inside_work_tree_false() {
        let temp_dir = TempDir::new().unwrap();
        assert!(!Git::inside_work_tree(&PathBuf::from(temp_dir.as_ref())));
        temp_dir.close().unwrap();
    }

    #[test]
    fn test_git_get_author_defaults() {
        // case 1: no author set, defaults set
        // set env to disable global git config
        env::set_var("GIT_CONFIG_GLOBAL", "/dev/null");
        let temp_dir = git_init_temp();
        let (name, email) = Git::get_author(
            &PathBuf::from(temp_dir.as_ref()),
            (
                Some("defaultname".to_string()),
                Some("defaultemail".to_string()),
            ),
        );
        assert_eq!(name, Some("defaultname".to_string()));
        assert_eq!(email, Some("defaultemail".to_string()));
    }

    #[test]
    fn test_git_get_author_git() {
        // case 2: author set, defaults set

        let temp_dir = git_init_temp();
        let git_user = "git_username";
        let git_email = "git_email";
        let gitconfig_path = temp_dir.path().join(".git").join("config");

        fs::write(
            &gitconfig_path,
            format!("[user]\n    name = {}\n    email = {}", git_user, git_email).as_str(),
        )
        .unwrap();
        let (name, email) = Git::get_author(
            &PathBuf::from(temp_dir.as_ref()),
            (
                Some("defaultname".to_string()),
                Some("defaultemail".to_string()),
            ),
        );
        assert_eq!(name, Some(git_user.to_string()));
        assert_eq!(email, Some(git_email.to_string()));
    }
}
