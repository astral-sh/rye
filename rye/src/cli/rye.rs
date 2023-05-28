use std::borrow::Cow;
use std::env::consts::{ARCH, EXE_EXTENSION, OS};
use std::env::{join_paths, split_paths};
use std::path::Path;
use std::process::Command;
use std::{env, fs};

use anyhow::{bail, Context, Error};
use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use console::style;
use minijinja::render;
use same_file::is_same_file;
use self_replace::self_delete_outside_path;
use tempfile::tempdir;

use crate::bootstrap::{download_url, ensure_self_venv, update_core_shims};
use crate::platform::{get_app_dir, symlinks_supported};
use crate::utils::{CommandOutput, QuietExit};

#[cfg(windows)]
const DEFAULT_HOME: &str = "%USERPROFILE%\\.rye";
#[cfg(unix)]
const DEFAULT_HOME: &str = "$HOME/.rye";

const GITHUB_REPO: &str = "https://github.com/mitsuhiko/rye";
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
    /// Update to a specific version.
    #[arg(long)]
    version: Option<String>,
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
pub struct InstallCommand {
    /// Skip prompts.
    #[arg(short, long)]
    yes: bool,
}

#[derive(Debug, Copy, Clone)]
enum InstallMode {
    Default,
    NoPrompts,
    AutoInstall,
}

/// Uninstalls rye again.
#[derive(Parser, Debug)]
pub struct UninstallCommand {
    /// Skip safety check.
    #[arg(short, long)]
    yes: bool,
}

#[derive(Parser, Debug)]
enum SubCommand {
    Completion(CompletionCommand),
    Update(UpdateCommand),
    #[command(hide = true)]
    Install(InstallCommand),
    Uninstall(UninstallCommand),
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    match cmd.command {
        SubCommand::Completion(args) => completion(args),
        SubCommand::Update(args) => update(args),
        SubCommand::Install(args) => install(args),
        SubCommand::Uninstall(args) => uninstall(args),
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
    // git based installation with cargo
    if args.rev.is_some() || args.tag.is_some() {
        let mut cmd = Command::new("cargo");
        let tmp = tempdir()?;
        cmd.arg("install")
            .arg("--git")
            .arg("https://github.com/mitsuhiko/rye")
            .arg("--root")
            .env(
                "PATH",
                join_paths(
                    Some(tmp.path().join("bin"))
                        .into_iter()
                        .chain(split_paths(&env::var_os("PATH").unwrap_or_default())),
                )?,
            )
            .arg(tmp.path());
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
        update_exe_and_shims(
            &tmp.path()
                .join("bin")
                .join("rye")
                .with_extension(EXE_EXTENSION),
        )?;
    } else {
        let version = args.version.as_deref().unwrap_or("latest");
        eprintln!("Updating to {version}");
        let binary = format!("rye-{ARCH}-{OS}");
        let ext = if cfg!(unix) { ".gz" } else { ".exe" };
        let url = if version == "latest" {
            format!("{GITHUB_REPO}/releases/latest/download/{binary}{ext}")
        } else {
            format!("{GITHUB_REPO}/releases/download/{version}/{binary}{ext}")
        };
        let bytes = download_url(&url, CommandOutput::Normal)
            .with_context(|| format!("could not download {version} release for this platform"))?;
        let tmp = tempfile::NamedTempFile::new()?;

        // unix currently comes compressed, windows comes uncompressed
        #[cfg(unix)]
        {
            use std::io::Read;
            let mut decoder = flate2::bufread::GzDecoder::new(&bytes[..]);
            let mut rv = Vec::new();
            decoder.read_to_end(&mut rv)?;
            fs::write(tmp.path(), rv)?;
        }
        #[cfg(windows)]
        {
            fs::write(tmp.path(), bytes)?;
        }
        update_exe_and_shims(tmp.path())?;
    }

    eprintln!("Updated!");
    eprintln!();
    Command::new(env::current_exe()?)
        .arg("--version")
        .status()?;

    Ok(())
}

fn update_exe_and_shims(new_exe: &Path) -> Result<(), Error> {
    let app_dir = get_app_dir().canonicalize()?;
    let current_exe = env::current_exe()?.canonicalize()?;
    let shims = app_dir.join("shims");

    self_replace::self_replace(new_exe)?;

    // if the shims have been created before (they really should have)
    // we want to make sure that they point to the new executable now.
    // for symlinks that probably is not necessary, but for hardlinks
    // that's very important.
    if shims.is_dir() {
        update_core_shims(&shims, &current_exe)?;
    }

    Ok(())
}

fn install(args: InstallCommand) -> Result<(), Error> {
    perform_install(if args.yes {
        InstallMode::NoPrompts
    } else {
        InstallMode::Default
    })
}

fn remove_dir_all_if_exists(path: &Path) -> Result<(), Error> {
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

fn uninstall(args: UninstallCommand) -> Result<(), Error> {
    if !args.yes
        && !dialoguer::Confirm::new()
            .with_prompt("Do you want to uninstall rye?")
            .interact()?
    {
        return Ok(());
    }

    let app_dir = get_app_dir();
    if app_dir.is_dir() {
        let real_exe = env::current_exe()?.canonicalize()?;
        let real_app_dir = app_dir.canonicalize()?;

        // try to delete all shims that can be found.  Ignore if deletes don't work.
        // The delete of the current executable for instance will fail on windows.
        let shim_dir = app_dir.join("shims");
        if let Ok(dir) = shim_dir.read_dir() {
            for entry in dir.flatten() {
                fs::remove_file(&entry.path()).ok();
            }
        }

        remove_dir_all_if_exists(&app_dir.join("self"))?;
        remove_dir_all_if_exists(&app_dir.join("py"))?;
        remove_dir_all_if_exists(&app_dir.join("pip-tools"))?;

        // special deleting logic if we are placed in the app dir and the shim deletion
        // did not succeed.  This is likely the case on windows where we then use the
        // `self_delete` crate.
        if real_exe.strip_prefix(&real_app_dir).is_ok() && real_exe.is_file() {
            self_delete_outside_path(&real_app_dir)?;
        }

        // at this point the remaining shim folder should be deletable
        remove_dir_all_if_exists(&app_dir.join("shims"))?;

        // leave this empty behind in case someone sourced it.  The config also stays around.
        let env_file = app_dir.join("env");
        if env_file.is_file() {
            fs::write(env_file, "")?;
        }
    }

    eprintln!("Done!");
    eprintln!();

    let rye_home = env::var("RYE_HOME")
        .map(Cow::Owned)
        .unwrap_or(Cow::Borrowed(DEFAULT_HOME));
    if cfg!(unix) {
        eprintln!(
            "Don't forget to remove the sourcing of {} from your shell config.",
            Path::new(&rye_home as &str).join("env").display()
        );
    } else {
        eprintln!(
            "Don't forget to remove {} from your PATH",
            Path::new(&rye_home as &str).join("shims").display()
        )
    }

    Ok(())
}

fn perform_install(mode: InstallMode) -> Result<(), Error> {
    let exe = env::current_exe()?;
    let app_dir = get_app_dir();
    let shims = app_dir.join("shims");
    let target = shims.join("rye").with_extension(EXE_EXTENSION);

    eprintln!("{}", style("Welcome to Rye!").bold());

    if matches!(mode, InstallMode::AutoInstall) {
        eprintln!();
        eprintln!("Rye has detected that it's not installed on this computer yet and");
        eprintln!("automatically started the installer for you.  For more information");
        eprintln!(
            "read {}",
            style("https://rye-up.com/guide/installation/").yellow()
        );
    }

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

    if cfg!(windows) && !symlinks_supported() {
        eprintln!();
        eprintln!(
            "{}: your Windows configuration does not support symlinks.",
            style("Warning").red()
        );
        eprintln!();
        eprintln!("It's strongly recommended that you enable developer mode in Windows to");
        eprintln!("enable symlinks.  You need to enable this before continuing the setup.");
        eprintln!(
            "Learn more at {}",
            style("https://rye-up.com/guide/faq/#windows-developer-mode").yellow()
        );
    }

    eprintln!();
    if !matches!(mode, InstallMode::NoPrompts)
        && !dialoguer::Confirm::new()
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
    eprintln!("Installed binary to {}", style(target.display()).cyan());

    // write an env file we can source later.  Prefer $HOME/.rye over
    // the expanded path, if not overridden.
    let (custom_home, rye_home) = env::var("RYE_HOME")
        .map(|x| (true, Cow::Owned(x)))
        .unwrap_or((false, Cow::Borrowed(DEFAULT_HOME)));

    if cfg!(unix) {
        fs::write(
            app_dir.join("env"),
            render!(UNIX_ENV_FILE, custom_home, rye_home),
        )?;
    }

    // Ensure internals next
    let self_path = ensure_self_venv(CommandOutput::Normal)?;
    eprintln!(
        "Updated self-python installation at {}",
        style(self_path.display()).cyan()
    );

    if cfg!(unix) {
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
            eprintln!(
                "Note: after adding rye to your path, restart your shell for it to take effect."
            );
        }
    } else if cfg!(windows) {
        eprintln!();
        eprintln!("Note: You need to manually add {DEFAULT_HOME} to your PATH.");
    }

    eprintln!("For more information read https://mitsuhiko.github.io/rye/guide/installation");

    eprintln!();
    eprintln!("{}", style("All done!").green());

    Ok(())
}

pub fn auto_self_install() -> Result<bool, Error> {
    // disables self installation
    if env::var("RYE_NO_AUTO_INSTALL").ok().as_deref() == Some("1") {
        return Ok(false);
    }

    let app_dir = get_app_dir();
    let rye_exe = app_dir
        .join("shims")
        .join("rye")
        .with_extension(EXE_EXTENSION);

    // it's already installed, don't install
    if app_dir.is_dir() && rye_exe.is_file() {
        Ok(false)
    } else {
        // in auto installation we want to show a continue prompt before we shut down
        // so that the cmd.exe does not close.
        #[cfg(windows)]
        {
            crate::request_continue_prompt();
        }

        perform_install(InstallMode::AutoInstall)?;
        Ok(true)
    }
}
