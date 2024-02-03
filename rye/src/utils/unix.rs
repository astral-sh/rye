use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{Context, Error};

pub(crate) fn add_to_path(rye_home: &Path) -> Result<(), Error> {
    // for regular shells just add the path to `.profile`
    add_source_line_to_profile(
        &home::home_dir()
            .context("could not find home dir")?
            .join(".profile"),
        &(format!(
            ". \"{}\"",
            reverse_resolve_env_home(rye_home.join("env")).display()
        )),
    )?;
    Ok(())
}

fn add_source_line_to_profile(profile_path: &Path, source_line: &str) -> Result<(), Error> {
    let mut profile = if profile_path.is_file() {
        fs::read_to_string(profile_path)?
    } else {
        String::new()
    };

    if !profile.lines().any(|x| x.trim() == source_line) {
        profile.push_str(source_line);
        profile.push('\n');
        fs::write(profile_path, profile).context("failed to write updated .profile")?;
    }

    Ok(())
}

fn reverse_resolve_env_home(path: PathBuf) -> PathBuf {
    if let Some(env_home) = env::var_os("HOME").map(PathBuf::from) {
        if let Ok(rest) = path.strip_prefix(&env_home) {
            return Path::new("$HOME").join(rest);
        }
    }
    path
}
