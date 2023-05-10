use std::borrow::Cow;
use std::io::Cursor;
use std::path::Path;
use std::{fmt, fs};

use anyhow::Error;
use once_cell::sync::Lazy;
use pep508_rs::{Requirement, VersionOrUrl};
use regex::{Captures, Regex};

static ENV_VAR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$\{([A-Z0-9_]+)\}").unwrap());

#[cfg(unix)]
pub use std::os::unix::fs::{symlink as symlink_file, symlink as symlink_dir};
#[cfg(windows)]
pub use std::os::windows::fs::{symlink_dir, symlink_file};

#[derive(Debug)]
pub struct QuietExit(pub i32);

impl std::error::Error for QuietExit {}

impl fmt::Display for QuietExit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "exit with {}", self.0)
    }
}

/// Controls the fetch output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum CommandOutput {
    /// Regular output
    #[default]
    Normal,
    /// Extra verbose output
    Verbose,
    /// No output
    Quiet,
}

impl CommandOutput {
    /// Returns the preferred command output for those flags.
    pub fn from_quiet_and_verbose(quiet: bool, verbose: bool) -> CommandOutput {
        if quiet {
            CommandOutput::Quiet
        } else if verbose {
            CommandOutput::Verbose
        } else {
            CommandOutput::Normal
        }
    }
}

pub fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::prelude::MetadataExt;
        path.metadata().map_or(false, |x| x.mode() & 0o001 != 0)
    }
    #[cfg(windows)]
    {
        path.with_extension("exe").is_file() || path.with_extension("bat").is_file()
    }
}

pub fn get_short_executable_name(path: &Path) -> String {
    #[cfg(unix)]
    {
        path.file_name().unwrap().to_string_lossy().to_string()
    }
    #[cfg(windows)]
    {
        let short_name = path.file_name().unwrap().to_string_lossy().to_lowercase();
        if let Some(script) = short_name.strip_suffix(".exe") {
            script.into()
        } else if let Some(script) = short_name.strip_suffix(".bat") {
            script.into()
        } else {
            short_name
        }
    }
}

/// Formats a Python requirement.
pub fn format_requirement(req: &Requirement) -> impl fmt::Display + '_ {
    struct Helper<'x>(&'x Requirement);

    impl<'x> fmt::Display for Helper<'x> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0.name)?;
            if let Some(extras) = &self.0.extras {
                write!(f, "[{}]", extras.join(","))?;
            }
            if let Some(version_or_url) = &self.0.version_or_url {
                match version_or_url {
                    VersionOrUrl::VersionSpecifier(version_specifier) => {
                        let version_specifier: Vec<String> =
                            version_specifier.iter().map(ToString::to_string).collect();
                        write!(f, "{}", version_specifier.join(", "))?;
                    }
                    VersionOrUrl::Url(url) => {
                        // retain `{` and `}` for interpolation in URLs
                        write!(
                            f,
                            " @ {}",
                            url.to_string().replace("%7B", "{").replace("%7D", "}")
                        )?;
                    }
                }
            }
            if let Some(marker) = &self.0.marker {
                write!(f, " ; {}", marker)?;
            }
            Ok(())
        }
    }

    Helper(req)
}

/// Helper to expand envvars
pub fn expand_env_vars<F>(string: &str, mut f: F) -> Cow<'_, str>
where
    F: for<'a> FnMut(&'a str) -> Option<String>,
{
    ENV_VAR_RE.replace_all(string, |m: &Captures| f(&m[1]).unwrap_or_default())
}

/// Unpacks a tarball.
///
/// Today this assumes that the tarball is zstd compressed which happens
/// to be what the indygreg python builds use.
pub fn unpack_tarball(contents: &[u8], dst: &Path, strip_components: usize) -> Result<(), Error> {
    let reader = Cursor::new(contents);
    let decoder = zstd::stream::read::Decoder::with_buffer(reader)?;
    let mut archive = tar::Archive::new(decoder);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let name = entry.path()?;
        let mut components = name.components();
        for _ in 0..strip_components {
            components.next();
        }
        let path = dst.join(components.as_path());

        // only unpack if it's save to do so
        if path != Path::new("") && path.strip_prefix(dst).is_ok() {
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir).ok();
            }
            entry.unpack(&path)?;
        }
    }
    Ok(())
}

#[test]
fn test_quiet_exit_display() {
    let quiet_exit = QuietExit(0);
    assert_eq!("exit with 0", format!("{}", quiet_exit));
}

#[cfg(test)]
mod test_format_requirement {
    use super::{format_requirement, Requirement};

    #[test]
    fn test_format_requirement_simple() {
        let req: Requirement = "foo>=1.0.0".parse().unwrap();
        assert_eq!("foo>=1.0.0", format_requirement(&req).to_string());
    }

    #[test]
    fn test_format_requirement_complex() {
        let req: Requirement = "foo[extra1,extra2]>=1.0.0,<2.0.0; python_version<'3.8'"
            .parse()
            .unwrap();
        assert_eq!(
            "foo[extra1,extra2]>=1.0.0, <2.0.0 ; python_version < '3.8'",
            format_requirement(&req).to_string()
        );
    }
    #[test]
    fn test_format_requirement_file_path() {
        // this support is just for generating dependencies.  Parsing such requirements
        // is only partially supported as expansion has to happen before parsing.
        let req: Requirement = "foo @ file:///${PROJECT_ROOT}/foo".parse().unwrap();
        assert_eq!(
            format_requirement(&req).to_string(),
            "foo @ file:///${PROJECT_ROOT}/foo"
        );
    }
}

#[cfg(test)]
mod test_command_output {
    use super::CommandOutput;

    #[test]
    fn test_command_output_defaults() {
        assert_eq!(CommandOutput::Normal, CommandOutput::default());
    }

    #[test]
    fn test_command_output_from_quiet_and_verbose() {
        let quiet = true;
        let verbose = true;

        assert_eq!(
            CommandOutput::Quiet,
            CommandOutput::from_quiet_and_verbose(quiet, false)
        );
        assert_eq!(
            CommandOutput::Verbose,
            CommandOutput::from_quiet_and_verbose(false, verbose)
        );
        assert_eq!(
            CommandOutput::Normal,
            CommandOutput::from_quiet_and_verbose(false, false)
        );
        assert_eq!(
            CommandOutput::Quiet,
            CommandOutput::from_quiet_and_verbose(quiet, verbose)
        ); // Quiet takes precedence over verbose
    }
}

#[cfg(test)]
mod test_expand_env_vars {
    use super::expand_env_vars;

    #[test]
    fn test_expand_env_vars_no_expansion() {
        let input = "This string has no env vars";
        let output = expand_env_vars(input, |_| None);
        assert_eq!(input, output);
    }

    #[test]
    fn test_expand_env_vars_with_expansion() {
        let input = "This string has an env var: ${EXAMPLE_VAR}";
        let output = expand_env_vars(input, |var| {
            if var == "EXAMPLE_VAR" {
                Some("Example value".to_string())
            } else {
                None
            }
        });
        assert_eq!("This string has an env var: Example value", output);
    }
}
