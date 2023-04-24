use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::{env, fmt, fs};

use anyhow::Error;
use pep508_rs::{Requirement, VersionOrUrl};

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
                        write!(f, " @ {}", url)?;
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

/// Returns the current exe.
pub fn current_exe_portable() -> Result<PathBuf, Error> {
    #[cfg(target_os = "linux")]
    {
        // support symlinks by using args[0], when possible, with fallback to current_exe()
        if let Some(ref s) = env::args_os().next() {
            if !s.is_empty() {
                return Ok(PathBuf::from(s));
            }
        }
    }
    Ok(env::current_exe()?)
}
