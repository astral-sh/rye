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

use crate::bootstrap::{download_url, ensure_self_venv};
use crate::platform::get_app_dir;
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
    // git based installation with cargo
    if args.rev.is_some() || args.tag.is_some() {
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

        self_replace::self_replace(tmp.path())?;
        eprintln!("Updated!");
        eprintln!();
        Command::new(env::current_exe()?)
            .arg("--version")
            .status()?;
    }

    Ok(())
}

fn install(_args: InstallCommand) -> Result<(), Error> {
    let exe = env::current_exe()?;
    let app_dir = get_app_dir();
    let shims = app_dir.join("shims");
    let target = shims.join("rye").with_extension(EXE_EXTENSION);

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

    eprintln!();
    eprintln!("{}", style("All done!").green());

    Ok(())
}
