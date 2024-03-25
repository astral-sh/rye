use std::borrow::Cow;
use std::env::consts::{ARCH, EXE_EXTENSION, OS};
use std::env::{join_paths, split_paths};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::{env, fs};

use anyhow::{anyhow, bail, Context, Error};
use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use console::style;
use minijinja::render;
use self_replace::self_delete_outside_path;
use tempfile::tempdir;

use crate::bootstrap::{
    download_url, download_url_ignore_404, ensure_self_venv_with_toolchain,
    is_self_compatible_toolchain, update_core_shims, SELF_PYTHON_TARGET_VERSION,
};
use crate::cli::toolchain::register_toolchain;
use crate::config::Config;
use crate::platform::{get_app_dir, symlinks_supported};
use crate::sources::py::{get_download_url, PythonVersionRequest};
use crate::utils::{check_checksum, toml, tui_theme, CommandOutput, IoPathContext, QuietExit};

#[cfg(windows)]
const DEFAULT_HOME: &str = "%USERPROFILE%\\.rye";
#[cfg(unix)]
const DEFAULT_HOME: &str = "$HOME/.rye";

const GITHUB_REPO: &str = "https://github.com/astral-sh/rye";
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
/// This can install updates from the latest release binaries or trigger a manual
/// compilation of Rye if Rust is installed.
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
    /// Update to a specific git branch.
    #[arg(long, conflicts_with = "tag", conflicts_with = "rev")]
    branch: Option<String>,
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
    /// Register a specific toolchain before bootstrap.
    #[arg(long)]
    toolchain: Option<PathBuf>,
    /// Use a specific toolchain version.
    #[arg(long)]
    toolchain_version: Option<PythonVersionRequest>,

    #[command(flatten)]
    mp: ModifyPath,
}

#[derive(Parser, Debug)]
#[group(required = false, multiple = false)]
pub struct ModifyPath {
    /// Always modify without asking the PATH environment variable.
    #[arg(long)]
    modify_path: bool,
    /// Do not modify the PATH environment variable.
    #[arg(long)]
    no_modify_path: bool,
}

#[derive(Debug, Copy, Clone)]
enum YesNoArg {
    Yes,
    No,
    Ask,
}

impl YesNoArg {
    fn with_yes(&self, yes: bool) -> Self {
        match (yes, self) {
            (true, Self::Ask) => Self::Yes,
            _ => *self,
        }
    }
}
impl From<ModifyPath> for YesNoArg {
    fn from(other: ModifyPath) -> Self {
        // Argument parsing logic is a bit complex here:
        match (other.modify_path, other.no_modify_path) {
            // 1. If --modify-path is set and --no-modify-path is not set, we always modify the path without prompting.
            (true, false) => YesNoArg::Yes,
            // 2. If --no-modify-path is set and --modify-path is not set, we never modify the path.
            (false, true) => YesNoArg::No,
            // 3. Otherwise we ask the user
            (false, false) => YesNoArg::Ask,
            (true, true) => unreachable!(),
        }
    }
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
    // make sure to read the exe before self_replace as otherwise we might read
    // a bad executable name on Linux where the move is picked up.
    let current_exe = env::current_exe()?;

    // git based installation with cargo
    if args.rev.is_some() || args.tag.is_some() || args.branch.is_some() {
        let mut cmd = Command::new("cargo");
        let tmp = tempdir()?;
        cmd.arg("install")
            .arg("--git")
            .arg("https://github.com/astral-sh/rye")
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
        } else if let Some(ref branch) = args.branch {
            cmd.arg("--branch");
            cmd.arg(branch);
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
        )
    } else {
        let version = args.version.as_deref().unwrap_or("latest");
        echo!("Updating to {version}");
        let binary = format!("rye-{ARCH}-{OS}");
        let ext = if cfg!(unix) { ".gz" } else { ".exe" };
        let url = if version == "latest" {
            format!("{GITHUB_REPO}/releases/latest/download/{binary}{ext}")
        } else {
            format!("{GITHUB_REPO}/releases/download/{version}/{binary}{ext}")
        };
        let sha256_url = format!("{}.sha256", url);
        let bytes = download_url(&url, CommandOutput::Normal)
            .with_context(|| format!("could not download release {version} for this platform"))?;
        if let Some(sha256_bytes) = download_url_ignore_404(&sha256_url, CommandOutput::Normal)? {
            let checksum = String::from_utf8_lossy(&sha256_bytes);
            echo!("Checking checksum");
            check_checksum(&bytes, checksum.trim())
                .with_context(|| format!("hash check of {} failed", url))?;
        } else {
            echo!("Checksum check skipped (no hash available)");
        }

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
        update_exe_and_shims(tmp.path())
    }
    .context(
        "Unable to perform update. This can happen because files are in use. \
         Please stop running Python interpreters and retry the update.",
    )?;

    echo!("Validate updated installation");
    validate_updated_exe(&current_exe)
        .context("unable to perform validation of updated installation")?;

    echo!("Updated!");
    echo!();
    Command::new(current_exe).arg("--version").status()?;

    Ok(())
}

fn validate_updated_exe(rye: &Path) -> Result<(), Error> {
    let folder = tempfile::tempdir()?;

    // first create a dummy project via the new rye version
    if !Command::new(rye)
        .arg("init")
        .arg("--name=test-project")
        .arg("-q")
        .arg(".")
        .current_dir(folder.path())
        .status()?
        .success()
    {
        bail!("failed to initialize test project");
    }

    // then try to run the python shim in the context of that project.
    // this as a by product should update outdated internals and perform
    // a python only sync in all versions of rye known currently.
    if !Command::new(
        get_app_dir()
            .join("shims")
            .join("python")
            .with_extension(EXE_EXTENSION),
    )
    .arg("-c")
    .arg("")
    .current_dir(folder.path())
    .status()?
    .success()
    {
        bail!("failed to run python shim in test project");
    }

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
    perform_install(
        if args.yes {
            InstallMode::NoPrompts
        } else {
            InstallMode::Default
        },
        args.toolchain.as_deref(),
        args.toolchain_version,
        YesNoArg::from(args.mp).with_yes(args.yes),
    )
}

fn remove_dir_all_if_exists(path: &Path) -> Result<(), Error> {
    if path.is_dir() {
        fs::remove_dir_all(path).path_context(path, "failed to remove directory")?;
    }
    Ok(())
}

fn uninstall(args: UninstallCommand) -> Result<(), Error> {
    if !args.yes
        && !dialoguer::Confirm::with_theme(tui_theme())
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
        remove_dir_all_if_exists(&app_dir.join("uv"))?;
        remove_dir_all_if_exists(&app_dir.join("tools"))?;

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

    echo!("Done!");
    echo!();

    let rye_home = env::var("RYE_HOME")
        .map(Cow::Owned)
        .unwrap_or(Cow::Borrowed(DEFAULT_HOME));

    #[cfg(unix)]
    {
        echo!(
            "Don't forget to remove the sourcing of {} from your shell config.",
            Path::new(&*rye_home).join("env").display()
        );
    }

    #[cfg(windows)]
    {
        crate::utils::windows::remove_from_path(Path::new(&*rye_home))?;
        crate::utils::windows::remove_from_programs()?;
    }

    Ok(())
}

#[cfg(unix)]
fn has_fish() -> bool {
    use which::which;
    which("fish").is_ok()
}

#[cfg(unix)]
fn has_zsh() -> bool {
    use which::which;
    which("zsh").is_ok()
}

fn perform_install(
    mode: InstallMode,
    toolchain_path: Option<&Path>,
    toolchain_version: Option<PythonVersionRequest>,
    modify_path: YesNoArg,
) -> Result<(), Error> {
    let mut config = Config::current();
    let mut registered_toolchain: Option<PythonVersionRequest> = None;
    let config_doc = Arc::make_mut(&mut config).doc_mut();
    let exe = env::current_exe()?;
    let app_dir = get_app_dir();
    let shims = app_dir.join("shims");
    let target = shims.join("rye").with_extension(EXE_EXTENSION);
    let mut prompt_for_toolchain_later = false;

    // When we perform an install and a toolchain path has not been passed,
    // we always also pick up on the RYE_TOOLCHAIN environment variable
    // as a fallback.
    let toolchain_path = match toolchain_path {
        Some(path) => Some(Cow::Borrowed(path)),
        None => env::var_os("RYE_TOOLCHAIN")
            .map(PathBuf::from)
            .map(Cow::Owned),
    };

    // Also pick up the target version from the RYE_TOOLCHAIN_VERSION
    // environment variable.
    let toolchain_version_request = match toolchain_version {
        Some(version) => Some(version),
        None => match env::var("RYE_TOOLCHAIN_VERSION") {
            Ok(val) => Some(val.parse()?),
            Err(_) => None,
        },
    };

    echo!("{}", style("Welcome to Rye!").bold());

    if matches!(mode, InstallMode::AutoInstall) {
        echo!();
        echo!("Rye has detected that it's not installed on this computer yet and");
        echo!("automatically started the installer for you. For more information");
        echo!(
            "read {}",
            style("https://rye-up.com/guide/installation/").yellow()
        );
    }

    echo!();
    echo!(
        "This installer will install rye to {}",
        style(app_dir.display()).cyan()
    );
    echo!(
        "This path can be changed by exporting the {} environment variable.",
        style("RYE_HOME").cyan()
    );
    echo!();
    echo!("{}", style("Details:").bold());
    echo!("  Rye Version: {}", style(env!("CARGO_PKG_VERSION")).cyan());
    echo!("  Platform: {} ({})", style(OS).cyan(), style(ARCH).cyan());
    if let Some(ref toolchain_path) = toolchain_path {
        echo!(
            "  Internal Toolchain Path: {}",
            style(toolchain_path.display()).cyan()
        );
    }
    if let Some(ref toolchain_version_request) = toolchain_version_request {
        echo!(
            "  Internal Toolchain Version: {}",
            style(toolchain_version_request).cyan()
        );
    }

    if cfg!(windows) && !symlinks_supported() {
        echo!();
        warn!("your Windows configuration does not support symlinks.");
        echo!();
        echo!("It's strongly recommended that you enable developer mode in Windows to");
        echo!("enable symlinks. You need to enable this before continuing the setup.");
        echo!(
            "Learn more at {}",
            style("https://rye-up.com/guide/faq/#windows-developer-mode").yellow()
        );
    }

    echo!();
    if !matches!(mode, InstallMode::NoPrompts)
        && !dialoguer::Confirm::with_theme(tui_theme())
            .with_prompt("Continue?")
            .interact()?
    {
        elog!("Installation cancelled!");
        return Err(QuietExit(1).into());
    }

    // Use uv?
    if config_doc
        .get("behavior")
        .and_then(|x| x.get("use-uv"))
        .is_none()
        && !matches!(mode, InstallMode::NoPrompts)
    {
        let use_uv = dialoguer::Select::with_theme(tui_theme())
            .with_prompt("Select the preferred package installer")
            .item("uv (fast, recommended)")
            .item("pip-tools (slow, higher compatibility)")
            .default(0)
            .interact()?
            == 0;
        toml::ensure_table(config_doc, "behavior")["use-uv"] = toml_edit::value(use_uv);
    }

    // If the global-python flag is not in the settings, ask the user if they want to turn
    // on global shims upon installation.
    if config_doc
        .get("behavior")
        .and_then(|x| x.get("global-python"))
        .is_none()
        && (matches!(mode, InstallMode::NoPrompts)
            || dialoguer::Select::with_theme(tui_theme())
                .with_prompt("What should running `python` or `python3` do when you are not inside a Rye managed project?")
                .item("Run a Python installed and managed by Rye")
                .item("Run the old default Python (provided by your OS, pyenv, etc.)")
                .default(0)
                .interact()?
                == 0)
    {
        toml::ensure_table(config_doc, "behavior")["global-python"] = toml_edit::value(true);

        // configure the default toolchain.  If we are not using a pre-configured toolchain we
        // can ask now, otherwise we need to wait for the toolchain to be available before we
        // can fill in the default.
        if !matches!(mode, InstallMode::NoPrompts) {
            if toolchain_path.is_none() {
                prompt_for_default_toolchain(
                    toolchain_version_request
                        .clone()
                        .unwrap_or(SELF_PYTHON_TARGET_VERSION),
                    config_doc,
                )?;
            } else {
                prompt_for_toolchain_later = true;
            }
        }
    }

    // place executable in rye home folder
    fs::create_dir_all(&shims).ok();
    if target.is_file() {
        fs::remove_file(&target).path_context(&target, "failed to delete old executable")?;
    }
    fs::copy(&exe, &target).path_context(&exe, "failed to copy executable")?;
    echo!("Installed binary to {}", style(target.display()).cyan());

    // write an env file we can source later.  Prefer $HOME/.rye over
    // the expanded path, if not overridden.
    let (custom_home, rye_home) = env::var("RYE_HOME")
        .map(|x| (true, Cow::Owned(x)))
        .unwrap_or((false, Cow::Borrowed(DEFAULT_HOME)));

    if cfg!(unix) {
        let env_path = app_dir.join("env");
        fs::write(&env_path, render!(UNIX_ENV_FILE, custom_home, rye_home))
            .path_context(&env_path, "failed to write env file")?;
    }

    // Register a toolchain if provided.
    if let Some(toolchain_path) = toolchain_path {
        echo!(
            "Registering toolchain at {}",
            style(toolchain_path.display()).cyan()
        );
        let version = register_toolchain(&toolchain_path, None, |ver| {
            if ver.name != "cpython" {
                bail!("Only cpython toolchains are allowed, got '{}'", ver.name);
            } else if !is_self_compatible_toolchain(ver) {
                bail!(
                    "Toolchain {} is not version compatible for internal use.",
                    ver
                );
            }
            Ok(())
        })?;
        echo!("Registered toolchain as {}", style(&version).cyan());
        registered_toolchain = Some(version.into());
    }

    // Ensure internals next
    let self_path =
        ensure_self_venv_with_toolchain(CommandOutput::Normal, toolchain_version_request)?;
    echo!(
        "Updated self-python installation at {}",
        style(self_path.display()).cyan()
    );

    // now that the registered toolchain is available, prompt now.
    if prompt_for_toolchain_later {
        prompt_for_default_toolchain(registered_toolchain.unwrap(), config_doc)?;
    }

    match modify_path {
        YesNoArg::Yes => {
            add_rye_to_path(&mode, shims.as_path(), false)?;
        }
        YesNoArg::No => {
            echo!(
                "Skipping PATH modification. You will need to add {} to your PATH manually.",
                style(shims.display()).cyan()
            );
        }
        YesNoArg::Ask => {
            add_rye_to_path(&mode, shims.as_path(), true)?;
        }
    }

    echo!();
    echo!("{}", style("All done!").green());

    config.save()?;

    Ok(())
}

/// Add rye to the users path.
#[cfg_attr(windows, allow(unused_variables))]
fn add_rye_to_path(mode: &InstallMode, shims: &Path, ask: bool) -> Result<(), Error> {
    let rye_home = env::var("RYE_HOME")
        .map(Cow::Owned)
        .unwrap_or(Cow::Borrowed(DEFAULT_HOME));

    let rye_home = Path::new(&*rye_home);
    // For unices, we ask the user if they want rye to be added to PATH.
    // If they choose to do so, we add the "env" script to .profile.
    // See [`crate::utils::unix::add_to_path`].
    #[cfg(unix)]
    {
        if !env::split_paths(&env::var_os("PATH").unwrap())
            .any(|x| same_file::is_same_file(x, shims).unwrap_or(false))
        {
            echo!();
            echo!(
                "The rye directory {} was not detected on {}.",
                style(shims.display()).cyan(),
                style("PATH").cyan()
            );
            echo!("It is highly recommended that you add it.");

            if matches!(mode, InstallMode::NoPrompts)
                || !ask
                || dialoguer::Confirm::with_theme(tui_theme())
                    .with_prompt(format!(
                        "Should the installer add Rye to {} via .profile?",
                        style("PATH").cyan()
                    ))
                    .interact()?
            {
                crate::utils::unix::add_to_path(rye_home)?;
                echo!("Added to {}.", style("PATH").cyan());
                echo!(
                    "{}: for this to take effect you will need to restart your shell or run this manually:",
                    style("note").cyan()
                );
            } else {
                echo!(
                    "{}: did not manipulate the path. To make it work, add this to your .profile manually:",
                    style("note").cyan()
                );
            }

            echo!();
            echo!("    source \"{}/env\"", rye_home.display());
            echo!();
            if has_zsh() {
                echo!("To make it work with zsh, you might need to add this to your .zprofile:");
                echo!();
                echo!("    source \"{}/env\"", rye_home.display());
                echo!();
            }
            if has_fish() {
                echo!("To make it work with fish, run this once instead:");
                echo!();
                echo!(
                    "    set -Ua fish_user_paths \"{}/shims\"",
                    rye_home.display()
                );
                echo!();
            }
            echo!("For more information read https://rye-up.com/guide/installation/");
        }
    }
    // On Windows, we add the rye directory to the user's PATH unconditionally.
    #[cfg(windows)]
    {
        crate::utils::windows::add_to_programs(rye_home)?;
        crate::utils::windows::add_to_path(rye_home)?;
    }
    Ok(())
}

fn prompt_for_default_toolchain(
    default_toolchain: PythonVersionRequest,
    config_doc: &mut toml_edit::DocumentMut,
) -> Result<(), Error> {
    let choice = dialoguer::Input::with_theme(tui_theme())
        .with_prompt("Which version of Python should be used as default toolchain?")
        .default(default_toolchain.clone())
        .validate_with(move |version: &PythonVersionRequest| {
            // this is for ensuring that if a toolchain was registered manually we can
            // accept it, even if it's not downloadable
            if version == &default_toolchain {
                return Ok(());
            }
            get_download_url(version)
                .map(|_| ())
                .ok_or_else(|| anyhow!("Unavailable version '{}'", version))
        })
        .interact_text()?;
    toml::ensure_table(config_doc, "default")["toolchain"] = toml_edit::value(choice.to_string());
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

        perform_install(InstallMode::AutoInstall, None, None, YesNoArg::Yes)?;
        Ok(true)
    }
}
