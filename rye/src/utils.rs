use std::borrow::Cow;
use std::io::Cursor;
use std::path::Path;
use std::{fmt, fs};

use anyhow::Error;
use once_cell::sync::Lazy;
use pep508_rs::{Requirement, VersionOrUrl};
use regex::{Captures, Regex};

static ENV_VAR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$\{([A-Z0-9_]+)\}").unwrap());

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
fn test_format_requirement() {
    // this support is just for generating dependencies.  Parsing such requirements
    // is only partially supported as expansion has to happen before parsing.
    let req: Requirement = "foo @ file:///${PROJECT_ROOT}/foo".parse().unwrap();
    assert_eq!(
        format_requirement(&req).to_string(),
        "foo @ file:///${PROJECT_ROOT}/foo"
    );
}
