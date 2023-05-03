use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Error};
use clap::Parser;
use toml_edit::{Item, Table};

use crate::bootstrap::ensure_self_venv;
use crate::config::{get_credentials, write_credentials};
use crate::pyproject::PyProject;
use crate::utils::auth::{decrypt, encrypt};
use crate::utils::CommandOutput;

/// Publish packages to a package repository.
#[derive(Parser, Debug)]
pub struct Args {
    /// The distribution files to upload to the repository (defaults to <workspace-root>/dist/*).
    dist: Option<Vec<PathBuf>>,
    /// The repository to publish to (defaults to 'pypi').
    #[arg(short, long, default_value = "pypi")]
    repository: String,
    /// The repository url to publish to (defaults to https://upload.pypi.org/legacy/).
    #[arg(long, default_value = "https://upload.pypi.org/legacy/")]
    repository_url: String,
    /// An access token used for the upload.
    #[arg(long)]
    token: Option<String>,
    /// Sign files to upload using GPG.
    #[arg(long)]
    sign: bool,
    /// GPG identity used to sign files.
    #[arg(short, long)]
    identity: Option<String>,
    /// Path to alternate CA bundle.
    #[arg(long)]
    cert: Option<PathBuf>,
    /// Path to the .pypirc config file to use.
    #[arg(long)]
    config_file: Option<PathBuf>,
    /// Enables verbose diagnostics.
    #[arg(short, long)]
    verbose: bool,
    /// Turns off all output.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
}

pub fn execute(cmd: Args) -> Result<(), Error> {
    let output = CommandOutput::from_quiet_and_verbose(cmd.quiet, cmd.verbose);
    let venv = ensure_self_venv(output)?;
    let project = PyProject::discover()?;

    // Get the files to publish.
    let files = match cmd.dist {
        Some(paths) => paths,
        None => vec![project.workspace_path().join("dist").join("*")],
    };

    // a. Get token from arguments and offer encryption, then store in credentials file.
    // b. Get token from ~/.rye/credentials keyed by provided repository and provide decryption option.
    // c. Otherwise prompt for token and provide encryption option, storing the result in credentials.
    let repository = &cmd.repository;
    let mut credentials = get_credentials()?;
    credentials
        .entry(repository)
        .or_insert(Item::Table(Table::new()));

    let token = if let Some(token) = cmd.token {
        let encrypted_token = prompt_encrypt_with_passphrase(&token)?;
        credentials[repository]["token"] = Item::Value(encrypted_token.into());
        write_credentials(&credentials)?;

        token
    } else if let Some(token) = credentials
        .get(repository)
        .and_then(|table| table.get("token"))
        .map(|token| token.to_string())
    {
        prompt_decrypt_with_passphrase(&token)?
    } else {
        eprintln!("No access token found, generate one at: https://pypi.org/manage/account/token/");
        let token = prompt_for_token()?;
        let encrypted_token = prompt_encrypt_with_passphrase(&token)?;
        credentials[repository]["token"] = Item::Value(encrypted_token.into());
        write_credentials(&credentials)?;

        token
    };

    let mut publish_cmd = Command::new(venv.join("bin/python"));
    publish_cmd
        .arg("-mtwine")
        .arg("--no-color")
        .arg("upload")
        .args(files)
        .arg("--user")
        .arg("__token__")
        .arg("--password")
        .arg(token)
        .arg("--repository-url")
        .arg(cmd.repository_url);
    if cmd.sign {
        publish_cmd.arg("--sign");
    }
    if let Some(identity) = cmd.identity {
        publish_cmd.arg("--identity").arg(identity);
    }
    if let Some(config_path) = cmd.config_file {
        publish_cmd.arg("--config-file").arg(config_path);
    }
    if let Some(cert) = cmd.cert {
        publish_cmd.arg("--cert").arg(cert);
    }

    if output == CommandOutput::Quiet {
        publish_cmd.stdout(Stdio::null());
        publish_cmd.stderr(Stdio::null());
    }

    let status = publish_cmd.status()?;
    if !status.success() {
        bail!("failed to publish files");
    }

    Ok(())
}

fn prompt_for_token() -> Result<String, Error> {
    eprint!("Access token: ");
    let token = get_trimmed_user_input().context("failed to read provided token")?;

    Ok(token)
}

fn prompt_encrypt_with_passphrase(s: &str) -> Result<String, Error> {
    eprint!("Enter a passphrase (optional): ");
    let phrase = get_trimmed_user_input().context("failed to read provided passphrase")?;

    let token = if phrase.is_empty() {
        s.to_string()
    } else {
        let bytes = encrypt(s.as_bytes(), &phrase)?;
        String::from_utf8(bytes).context("failed to parse utf-8 from bytes")?
    };

    Ok(token)
}

fn prompt_decrypt_with_passphrase(s: &str) -> Result<String, Error> {
    eprint!("Enter a passphrase (optional): ");
    let phrase = get_trimmed_user_input().context("failed to read provided passphrase")?;

    let token = if phrase.is_empty() {
        s.to_string()
    } else if let Some(bytes) = decrypt(s.as_bytes(), &phrase) {
        String::from_utf8(bytes).context("failed to parse utf-8 from bytes")?
    } else {
        bail!("failed to decrypt")
    };

    Ok(token)
}

fn get_trimmed_user_input() -> Result<String, Error> {
    std::io::stderr().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}
