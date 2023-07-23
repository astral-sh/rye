use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use age::{
    secrecy::{ExposeSecret, Secret},
    Decryptor, Encryptor,
};
use anyhow::{bail, Context, Error};
use clap::Parser;
use toml_edit::{Item, Table};
use url::Url;

use crate::bootstrap::ensure_self_venv;
use crate::platform::{get_credentials, write_credentials};
use crate::pyproject::PyProject;
use crate::utils::{escape_string, get_venv_python_bin, CommandOutput};

/// Publish packages to a package repository.
#[derive(Parser, Debug)]
pub struct Args {
    /// The distribution files to upload to the repository (defaults to <workspace-root>/dist/*).
    dist: Option<Vec<PathBuf>>,
    /// The repository to publish to.
    #[arg(short, long, default_value = "pypi")]
    repository: String,
    /// The repository url to publish to.
    #[arg(long)]
    repository_url: Option<Url>,
    /// The username to authenticate to the repository with.
    #[arg(short, long)]
    username: Option<String>,
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
    /// Skip prompts.
    #[arg(short, long)]
    yes: bool,
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

    let repository_url = match cmd.repository_url {
        Some(url) => url,
        None => {
            let default_repository_url = Url::parse("https://upload.pypi.org/legacy/")?;
            credentials
                .get(repository)
                .and_then(|table| table.get("repository-url"))
                .map(|url| match Url::parse(&escape_string(url.to_string())) {
                    Ok(url) => url,
                    Err(_) => default_repository_url.clone(),
                })
                .unwrap_or(default_repository_url)
        }
    };

    // If -r is pypi but the url isn't pypi then bail
    if repository == "pypi" && repository_url.domain() != Some("upload.pypi.org") {
        bail!("invalid pypi url {} (use -h for help)", repository_url);
    }

    let username = match cmd.username {
        Some(username) => username,
        None => credentials
            .get(repository)
            .and_then(|table| table.get("username"))
            .map(|username| username.to_string())
            .map(escape_string)
            .unwrap_or("__token__".to_string()),
    };

    let token = if let Some(token) = cmd.token {
        let secret = Secret::new(token);
        let maybe_encrypted = maybe_encrypt(&secret, cmd.yes)?;
        let maybe_encoded = maybe_encode(&secret, &maybe_encrypted);
        credentials[repository]["token"] = Item::Value(maybe_encoded.expose_secret().into());
        write_credentials(&credentials)?;

        secret
    } else if let Some(token) = credentials
        .get(repository)
        .and_then(|table| table.get("token"))
        .map(|token| token.to_string())
        .map(escape_string)
    {
        let secret = Secret::new(token);

        maybe_decrypt(&secret, cmd.yes)?
    } else {
        echo!("No access token found, generate one at: https://pypi.org/manage/account/token/");
        let token = if !cmd.yes {
            prompt_for_token()?
        } else {
            "".to_string()
        };
        if token.is_empty() {
            bail!("an access token is required")
        }
        let secret = Secret::new(token);
        let maybe_encrypted = maybe_encrypt(&secret, cmd.yes)?;
        let maybe_encoded = maybe_encode(&secret, &maybe_encrypted);
        credentials[repository]["token"] = Item::Value(maybe_encoded.expose_secret().into());

        secret
    };

    credentials[repository]["repository-url"] = Item::Value(repository_url.to_string().into());
    credentials[repository]["username"] = Item::Value(username.clone().into());
    write_credentials(&credentials)?;

    let mut publish_cmd = Command::new(get_venv_python_bin(&venv));
    publish_cmd
        .arg("-mtwine")
        .arg("--no-color")
        .arg("upload")
        .args(files)
        .arg("--username")
        .arg(username)
        .arg("--password")
        .arg(token.expose_secret())
        .arg("--repository-url")
        .arg(repository_url.to_string());
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

fn maybe_encrypt(secret: &Secret<String>, yes: bool) -> Result<Secret<Vec<u8>>, Error> {
    let phrase = if !yes {
        dialoguer::Password::new()
            .with_prompt("Encrypt with passphrase (optional)")
            .allow_empty_password(true)
            .report(false)
            .interact()
            .map(Secret::new)?
    } else {
        Secret::new("".to_string())
    };

    let token = if phrase.expose_secret().is_empty() {
        secret.expose_secret().as_bytes().to_vec()
    } else {
        // Do the encryption
        let encryptor = Encryptor::with_user_passphrase(phrase);
        let mut encrypted = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted)?;
        writer.write_all(secret.expose_secret().as_bytes())?;
        writer.finish()?;

        encrypted
    };

    Ok(Secret::new(token.to_vec()))
}

fn maybe_decrypt(secret: &Secret<String>, yes: bool) -> Result<Secret<String>, Error> {
    let phrase = if !yes {
        dialoguer::Password::new()
            .with_prompt("Decrypt with passphrase (optional)")
            .allow_empty_password(true)
            .report(false)
            .interact()
            .map(Secret::new)?
    } else {
        Secret::new("".to_string())
    };

    if phrase.expose_secret().is_empty() {
        return Ok(secret.clone());
    }

    // If a passphrase is provided we assume the secret is encoded bytes from encryption.
    let bytes = hex::decode(pad_hex(secret.expose_secret().clone()))?;
    if let Decryptor::Passphrase(decryptor) = Decryptor::new(bytes.as_slice())? {
        // Do the decryption
        let mut decrypted = vec![];
        let mut reader = decryptor.decrypt(&phrase, None)?;
        reader.read_to_end(&mut decrypted)?;

        let token = String::from_utf8(decrypted).context("failed to parse utf-8")?;
        let secret = Secret::new(token);

        return Ok(secret);
    }

    bail!("failed to decrypt")
}

fn get_trimmed_user_input() -> Result<String, Error> {
    std::io::stderr().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}

/// Helper function to manage potentially encoding secret data.
///
/// If the original secret data (bytes) are not the same as the new secret's
/// then we encode, assuming the new data is encrypted data. Otherwise return
/// a new secret with the same string.
fn maybe_encode(original_secret: &Secret<String>, new_secret: &Secret<Vec<u8>>) -> Secret<String> {
    if original_secret.expose_secret().as_bytes() != new_secret.expose_secret() {
        let encoded = hex::encode(new_secret.expose_secret());
        return Secret::new(encoded);
    }

    original_secret.clone()
}

fn pad_hex(s: String) -> String {
    if s.len() % 2 == 1 {
        format!("0{}", s)
    } else {
        s
    }
}
