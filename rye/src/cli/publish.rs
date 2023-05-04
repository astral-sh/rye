use std::io::Write;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Error};
use clap::Parser;
use ring::pbkdf2;
use ring::rand::{SecureRandom, SystemRandom};
use toml_edit::{Item, Table};

use crate::bootstrap::ensure_self_venv;
use crate::config::{get_credentials, write_credentials};
use crate::pyproject::PyProject;
use crate::utils::auth::Secret;
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
        let secret = Secret::from(token);
        let encrypted_token = prompt_encrypt_with_passphrase(&secret)?;
        credentials[repository]["token"] = Item::Value(encrypted_token.into());
        write_credentials(&credentials)?;

        secret.to_string()
    } else if let Some(token) = credentials
        .get(repository)
        .and_then(|table| table.get("token"))
        .map(|token| token.to_string())
    {
        let secret = Secret::from(token);
        prompt_decrypt_with_passphrase(&secret)?
    } else {
        eprintln!("No access token found, generate one at: https://pypi.org/manage/account/token/");
        let token = prompt_for_token()?;
        let secret = Secret::from(token);
        let encrypted_token = prompt_encrypt_with_passphrase(&secret)?;
        credentials[repository]["token"] = Item::Value(encrypted_token.into());
        write_credentials(&credentials)?;

        secret.to_string()
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

fn prompt_encrypt_with_passphrase(secret: &Secret<String>) -> Result<String, Error> {
    eprint!("Enter a passphrase (optional): ");
    let phrase = get_trimmed_user_input().context("failed to read provided passphrase")?;

    let token = if phrase.is_empty() {
        secret.to_string()
    } else {
        // Get a nonce and a key for encryption
        let mut nonce_data = [0; 12];
        SystemRandom::new().fill(&mut nonce_data).unwrap();
        let key = derive_key(&phrase, &nonce_data);

        // Encode the encrypted token
        let bytes = secret.encrypt_with_key(&key, nonce_data)?;

        hex::encode(bytes)
    };

    Ok(token)
}

fn prompt_decrypt_with_passphrase(secret: &Secret<String>) -> Result<String, Error> {
    eprint!("Enter a passphrase (optional): ");
    let phrase = get_trimmed_user_input().context("failed to read provided passphrase")?;

    if phrase.is_empty() {
        Ok(secret.to_string())
    } else {
        // Decode the encoded token
        let bytes = hex::decode(secret.to_string())?;
        let decoded = Secret::from(String::from_utf8(bytes)?);

        // Get a nonce and create a key for decryption
        let mut nonce_data = [0; 12];
        SystemRandom::new().fill(&mut nonce_data).unwrap();
        let key = derive_key(&phrase, &nonce_data);

        if let Some(bytes) = decoded.decrypt_with_key(&key, nonce_data) {
            let token = String::from_utf8(bytes).context("failed to parse utf-8 from bytes")?;

            Ok(token)
        } else {
            bail!("failed to decrypt");
        }
    }
}

fn get_trimmed_user_input() -> Result<String, Error> {
    std::io::stderr().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}

fn derive_key(passphrase: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    let iterations = 100_000;

    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        NonZeroU32::new(iterations).unwrap(),
        salt,
        passphrase.as_bytes(),
        &mut key,
    );

    key
}
