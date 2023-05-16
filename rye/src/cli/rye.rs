use std::borrow::Cow;
use std::env::consts::{ARCH, EXE_EXTENSION, OS};
use std::process::Command;
use std::{env, fs};

use anyhow::{bail, Context, Error};
use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use console::style;
use minijinja::render;
use same_file::is_same_file;

use crate::bootstrap::ensure_self_venv;
use crate::platform::get_app_dir;
use crate::utils::{CommandOutput, QuietExit};

const UNIX_ENV_FILE: &str = r#"
# rye shell setup
{%- if custom_home %}
export RYE_HOME="{{ rye_home }}"
{%- endif %}
case ":${PATH}:" in
  *:"{{ rye_home }}/shims":*)
    ;;
  *)
    export PATH="{{ rye_home }}/shims:$PATH"
    ;;
esac

"#;

/// Rye self management
#[derive(Parser, Debug)]
pub struct Args {
    #[command(subcommand)]
    command: SubCommand,
}

/// Generates a completion script for a shell.
#[derive(Parser, Debug)]
pub struct CompletionCommand {
    /// The shell to generate a completion script for (defaults to 'bash').
    #[arg(short, long)]
    shell: Option<Shell>,
}

/// Performs an update of rye.
///
/// This currently just is an alias to running cargo install again with the
/// right arguments.
#[derive(Parser, Debug)]
pub struct UpdateCommand {
    /// Update to a specific tag.
    #[arg(long)]
    tag: Option<String>,
    /// Update to a specific git rev.
    #[arg(long, conflicts_with = "tag")]
    rev: Option<String>,
    /// Force reinstallation
    #[arg(long)]
    force: bool,
}

/// Triggers the initial installation of Rye.
///
/// This command is executed by the installation step to move Rye
/// to the intended target location and to add Rye to the environment
/// variables.
#[derive(Parser, Debug)]
pub struct InstallCommand {}

#[derive(Parser, Debug)]
enum SubCommand {
    Completion(CompletionCommand),
    Update(UpdateCommand),
    #[command(hide = true)]
    Install(InstallCommand),
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    match cmd.command {
        SubCommand::Completion(args) => completion(args),
        SubCommand::Update(args) => update(args),
        SubCommand::Install(args) => install(args),
    }
}

fn completion(args: CompletionCommand) -> Result<(), Error> {
    clap_complete::generate(
        args.shell.unwrap_or(Shell::Bash),
        &mut super::Args::command(),
        "rye",
        &mut std::io::stdout(),
    );

    Ok(())
}

fn update(args: UpdateCommand) -> Result<(), Error> {
    let mut helper = rename_helper::RenameHelper::new()?;
    let mut cmd = Command::new("cargo");
    cmd.arg("install")
        .arg("--git")
        .arg("https://github.com/mitsuhiko/rye");
    if let Some(ref rev) = args.rev {
        cmd.arg("--rev");
        cmd.arg(rev);
    } else if let Some(ref tag) = args.tag {
        cmd.arg("--tag");
        cmd.arg(tag);
    }
    if args.force {
        cmd.arg("--force");
    }
    cmd.arg("rye");
    let status = cmd.status().context("unable to update via cargo-install")?;
    if !status.success() {
        bail!("failed to self-update via cargo-install");
    }
    helper.disarm();

    Ok(())
}

fn install(_args: InstallCommand) -> Result<(), Error> {
    let exe = env::current_exe()?;
    let app_dir = get_app_dir();
    let shims = app_dir.join("shims");
    let target = shims.join(format!("rye{EXE_EXTENSION}"));

    eprintln!("{}", style("Welcome to Rye!").bold());
    eprintln!();
    eprintln!(
        "This installer will install rye to {}",
        style(app_dir.display()).cyan()
    );
    eprintln!(
        "This path can be changed by exporting the {} environment variable.",
        style("RYE_HOME").cyan()
    );
    eprintln!();
    eprintln!("{}", style("Details:").bold());
    eprintln!("  Rye Version: {}", style(env!("CARGO_PKG_VERSION")).cyan());
    eprintln!("  Platform: {} ({})", style(OS).cyan(), style(ARCH).cyan());

    eprintln!();
    if !dialoguer::Confirm::new()
        .with_prompt("Continue?")
        .interact()?
    {
        eprintln!("Installation cancelled!");
        return Err(QuietExit(1).into());
    }

    // place executable in rye home folder
    fs::create_dir_all(&shims).ok();
    if target.is_file() {
        fs::remove_file(&target)?;
    }
    fs::copy(exe, &target)?;

    // write an env file we can source later.  Prefer $HOME/.rye over
    // the expanded path, if not overridden.
    let (custom_home, rye_home) = env::var("RYE_HOME")
        .map(|x| (true, Cow::Owned(x)))
        .unwrap_or((false, Cow::Borrowed("$HOME/.rye")));
    fs::write(
        app_dir.join("env"),
        render!(UNIX_ENV_FILE, custom_home, rye_home),
    )?;
    eprintln!("Installed binary to {}", style(target.display()).cyan());

    // Ensure internals next
    let self_path = ensure_self_venv(CommandOutput::Normal)?;
    eprintln!(
        "Updated self-python installation at {}",
        style(self_path.display()).cyan()
    );

    if !env::split_paths(&env::var_os("PATH").unwrap())
        .any(|x| is_same_file(x, &shims).unwrap_or(false))
    {
        eprintln!();
        eprintln!(
            "The rye directory {} was not detected on {}.",
            style(shims.display()).cyan(),
            style("PATH").cyan()
        );
        eprintln!("It is highly recommended that you add it.");
        eprintln!("Add this at the end of your .profile, .zprofile or similar:");
        eprintln!();
        eprintln!("    source \"{}/env\"", rye_home);
        eprintln!();
        eprintln!("Note: after adding rye to your path, restart your shell for it to take effect.");
    }

    eprintln!();
    eprintln!("{}", style("All done!").green());

    Ok(())
}

#[cfg(windows)]
mod rename_helper {
    use super::*;
    use std::{env, fs, path::PathBuf};

    pub struct RenameHelper {
        original_path: PathBuf,
        path: PathBuf,
        disarmed: bool,
    }

    impl RenameHelper {
        pub fn new() -> Result<RenameHelper, Error> {
            let original_path = env::current_exe()?;
            let path = original_path.with_extension("tmp");
            fs::rename(&original_path, &path)?;
            Ok(RenameHelper {
                original_path,
                path,
                disarmed: false,
            })
        }

        pub fn disarm(&mut self) {
            self.disarmed = true;
        }
    }

    impl Drop for RenameHelper {
        fn drop(&mut self) {
            if !self.disarmed {
                fs::rename(&self.path, &self.original_path).ok();
            }
        }
    }
}

#[cfg(unix)]
mod rename_helper {
    use super::*;
    pub struct RenameHelper;

    impl RenameHelper {
        pub fn new() -> Result<RenameHelper, Error> {
            Ok(RenameHelper)
        }

        pub fn disarm(&mut self) {}
    }
}
