use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use age::{
    secrecy::{ExposeSecret, Secret},
    Decryptor, Encryptor,
};
use anyhow::{bail, Context, Error};
use clap::Parser;
use toml_edit::{DocumentMut, Item, Table};
use url::Url;

use crate::bootstrap::ensure_self_venv;
use crate::platform::{get_credentials, write_credentials};
use crate::pyproject::PyProject;
use crate::utils::{escape_string, get_venv_python_bin, tui_theme, CommandOutput};

const DEFAULT_USERNAME: &str = "__token__";
const DEFAULT_REPOSITORY: &str = "pypi";
const DEFAULT_REPOSITORY_DOMAIN: &str = "upload.pypi.org";
const DEFAULT_REPOSITORY_URL: &str = "https://upload.pypi.org/legacy/";

/// Publish packages to a package repository.
#[derive(Parser, Debug)]
pub struct Args {
    /// The distribution files to upload to the repository (defaults to <workspace-root>/dist/*).
    dist: Option<Vec<PathBuf>>,
    /// The repository to publish to.
    #[arg(short, long)]
    repository: Option<String>,
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
    /// Skip files that have already been published (only applies to repositories supporting this feature)
    #[arg(long)]
    skip_existing: bool,
    /// Skip saving to credentials file.
    #[arg(long)]
    skip_save_credentials: bool,
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

    // Get the files to publish.
    let files = match cmd.dist {
        Some(paths) => paths,
        None => {
            let project = PyProject::discover()?;
            if project.is_virtual() {
                bail!("virtual packages cannot be published");
            }
            vec![project.workspace_path().join("dist").join("*")]
        }
    };

    // Resolve credentials file
    let mut credentials_file = get_credentials()?;
    let entry = if let Some(key) = cmd.repository.as_ref() {
        Some(credentials_file.entry(key))
    } else if cmd.repository_url.is_none() {
        let default_repository = Repository::default();
        let key = default_repository
            .name
            .expect("default: pypi repository name");
        Some(credentials_file.entry(&key))
    } else {
        // We can't key data into the credentials with only a url
        None
    };
    let entry = entry.map(|it| it.or_insert(Item::Table(Table::new())));
    let credentials_table = entry.as_deref();

    let token = cmd.token.map(Secret::new);

    let mut credentials =
        resolve_credentials(credentials_table, cmd.username.as_ref(), token.as_ref());
    let mut repository = resolve_repository(credentials_table, cmd.repository, cmd.repository_url)?;

    // Token is from cli
    let mut should_encrypt = token.is_some();
    // We want to prompt decrypt any tokens from files and prompt encrypt any new inputs (cli)
    let should_decrypt =
        !should_encrypt && credentials_table.map_or(false, |it| it.get("token").is_some());

    // Fallback prompts
    let mut passphrase = None;

    if !cmd.yes {
        if credentials.password.is_none() {
            if is_unknown_repository(&repository) || is_default_repository(&repository) {
                echo!("No access token found, generate one at: https://pypi.org/manage/account/token/");
            }
            credentials.password = prompt_token()?;
            should_encrypt = credentials.password.is_some();
        }

        if should_encrypt {
            passphrase = prompt_encrypt_passphrase()?;
        } else if should_decrypt {
            passphrase = prompt_decrypt_passphrase()?;
        }

        if repository.url.is_none() {
            repository.url = prompt_repository_url()?;
        }
    }

    let config = PublishConfig {
        credentials,
        repository,
    };
    let config = config.resolve_with_defaults();

    if !config_is_ready(&config) {
        bail!(
            "failed to resolve configuration for repository '{}'",
            config.repository.name.unwrap_or_default()
        );
    }

    if !cmd.skip_save_credentials && config.repository.name.is_some() {
        save_rye_credentials(
            &mut credentials_file,
            &config.credentials,
            &config.repository,
            should_encrypt,
            passphrase.as_ref(),
        )?;
    }

    let mut publish_cmd = Command::new(get_venv_python_bin(&venv));

    // Build Twine command
    publish_cmd
        .arg("-mtwine")
        .arg("--no-color")
        .arg("upload")
        .arg("--non-interactive")
        .args(files);

    if let Some(usr) = config.credentials.username {
        publish_cmd.arg("--username").arg(usr);
    }
    if let Some(pwd) = config.credentials.password.as_ref() {
        publish_cmd.arg("--password");

        if should_decrypt && passphrase.is_some() {
            // Can expect passphrase due to the condition
            publish_cmd.arg(decrypt(pwd, &passphrase.expect("passphrase"))?.expose_secret());
        } else {
            publish_cmd.arg(pwd.expose_secret());
        }
    }
    if let Some(url) = config.repository.url.as_ref() {
        publish_cmd.arg("--repository-url").arg(url.to_string());
    }
    if cmd.sign {
        publish_cmd.arg("--sign");
    }
    if let Some(identity) = cmd.identity {
        publish_cmd.arg("--identity").arg(identity);
    }
    if let Some(cert) = cmd.cert {
        publish_cmd.arg("--cert").arg(cert);
    }
    if cmd.skip_existing {
        publish_cmd.arg("--skip-existing");
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

fn resolve_credentials(
    credentials_table: Option<&Item>,
    username: Option<&String>,
    password: Option<&Secret<String>>,
) -> Credentials {
    let mut credentials = Credentials {
        username: None,
        password: None,
    };

    if username.is_some() {
        credentials.username = username.cloned();
    } else {
        credentials.username = credentials_table
            .as_ref()
            .and_then(|it| it.get("username").map(Item::to_string).map(escape_string));
    }

    if password.is_some() {
        credentials.password = password.cloned();
    } else {
        credentials.password = credentials_table.as_ref().and_then(|it| {
            it.get("token")
                .map(Item::to_string)
                .map(escape_string)
                .map(Secret::new)
        });
    }

    if credentials.username.is_some() && credentials.password.is_some() {
        return credentials;
    }

    // Rye resolves tokens from the file or the cli. If a token was resolved
    // we can assume a default username of __token__.
    if credentials.password.is_some() && credentials.username.is_none() {
        credentials.username = Some(DEFAULT_USERNAME.to_string())
    }

    credentials
}

fn resolve_repository(
    credentials_table: Option<&Item>,
    name: Option<String>,
    url: Option<Url>,
) -> Result<Repository, Error> {
    let mut repository = Repository { name, url };

    if repository.url.is_some() {
        return Ok(repository);
    }

    if let Some(cred_url) = credentials_table.as_ref().and_then(|it| {
        it.get("repository-url")
            .map(Item::to_string)
            .map(escape_string)
    }) {
        repository.url = Some(Url::parse(&cred_url)?);
    }

    if repository.url.is_none()
        && repository
            .name
            .as_ref()
            .map_or(false, |it| it == DEFAULT_REPOSITORY)
    {
        repository.url = Some(Url::parse(DEFAULT_REPOSITORY_URL)?);
    }

    Ok(repository)
}

/// We need:
/// 1. username
/// 2. password (token)
/// 4. repository url
///
/// This can be configured with:
/// 1. credentials file
/// 2. cli
///
/// (1) cli -> (2) credentials file -> (3) keyring
//
/// Only token ('pypi'):
/// A token is resolved from either the cli or the credentials file.
/// If a repository name, url, and a username aren't provided, we can
/// default to 'pypi' configuration and save for next time with __token__
/// username.
///
/// Only url (keyring):
/// Only a repository url is provided. We can default to keyring settings
/// with __token__.
///
/// Using a repository name:
/// If a repository name is provided we would expect either sufficient
/// configuration from remaining sources or from the credentials file.
/// This includes an `is_keyring_ready` check.
struct PublishConfig {
    credentials: Credentials,
    repository: Repository,
}

impl PublishConfig {
    /// fallback defaults:
    /// 1. username (__token__)
    /// 2. repository name ('pypi')
    /// 3. repository url ('pypi')
    fn resolve_with_defaults(self) -> Self {
        Self {
            credentials: self.credentials.resolve_with_defaults(),
            repository: self.repository.resolve_with_defaults(),
        }
    }
}

fn config_is_ready(config: &PublishConfig) -> bool {
    (config.credentials.username.is_some()
        && config.credentials.password.is_some()
        && config.repository.url.is_some())
        || config_is_keyring_ready(config)
}

fn config_is_keyring_ready(config: &PublishConfig) -> bool {
    config.credentials.username.is_some() && config.repository.url.is_some()
}

struct Credentials {
    username: Option<String>,
    password: Option<Secret<String>>,
}

impl Credentials {
    fn resolve_with_defaults(self) -> Self {
        Self {
            username: self.username.or(Some(DEFAULT_USERNAME.to_string())),
            password: self.password,
        }
    }
}

struct Repository {
    name: Option<String>,
    url: Option<Url>,
}

impl Default for Repository {
    fn default() -> Self {
        Self {
            name: Some(DEFAULT_REPOSITORY.to_string()),
            url: Some(default_repository_url()),
        }
    }
}

impl Repository {
    fn resolve_with_defaults(self) -> Self {
        let name = self.name;
        let url = self.url;

        if name.is_none() && url.is_none() {
            return Self::default();
        }

        if url.is_none() && name.as_ref().map_or(false, |it| it == DEFAULT_REPOSITORY) {
            return Self {
                name,
                url: Some(default_repository_url()),
            };
        }

        Self { name, url }
    }
}

fn default_repository_url() -> Url {
    Url::parse(DEFAULT_REPOSITORY_URL).expect("default: pypi repository url")
}

fn is_unknown_repository(repository: &Repository) -> bool {
    repository.name.is_none() && repository.url.is_none()
}

fn is_default_repository(repository: &Repository) -> bool {
    repository
        .name
        .as_ref()
        .map_or(false, |it| it == DEFAULT_REPOSITORY)
        && repository
            .url
            .as_ref()
            .map_or(false, |it| it.domain() == Some(DEFAULT_REPOSITORY_DOMAIN))
}

fn save_rye_credentials(
    file: &mut DocumentMut,
    credentials: &Credentials,
    repository: &Repository,
    should_encrypt: bool,
    passphrase: Option<&Secret<String>>,
) -> Result<(), Error> {
    // We need a repository to key the credentials with
    let Some(name) = repository.name.as_ref() else {
        echo!("no repository found");
        echo!("skipping save credentials");
        return Ok(());
    };

    let table = file.entry(name).or_insert(Item::Table(Table::new()));

    if let Some(it) = credentials.password.as_ref() {
        let mut final_token = it.expose_secret().clone();
        if let Some(phrase) = passphrase.as_ref() {
            if should_encrypt {
                final_token = hex::encode(encrypt(it, phrase)?.expose_secret());
            }
        }
        if !final_token.is_empty() {
            table["token"] = Item::Value(final_token.into());
        }
    }

    if let Some(usr) = credentials.username.as_ref() {
        if !usr.is_empty() {
            table["username"] = Item::Value(usr.clone().into());
        }
    }

    if let Some(url) = repository.url.as_ref() {
        table["repository-url"] = Item::Value(url.to_string().into());
    }

    write_credentials(file)
}

fn prompt_token() -> Result<Option<Secret<String>>, Error> {
    eprint!("Access token: ");
    let token = get_trimmed_user_input().context("failed to read provided token")?;

    if token.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Secret::new(token)))
    }
}

fn prompt_encrypt_passphrase() -> Result<Option<Secret<String>>, Error> {
    let phrase = dialoguer::Password::with_theme(tui_theme())
        .with_prompt("Encrypt with passphrase (optional)")
        .allow_empty_password(true)
        .report(false)
        .interact()?;

    if phrase.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Secret::new(phrase)))
    }
}

fn encrypt(secret: &Secret<String>, phrase: &Secret<String>) -> Result<Secret<Vec<u8>>, Error> {
    let token = if phrase.expose_secret().is_empty() {
        secret.expose_secret().as_bytes().to_vec()
    } else {
        // Do the encryption
        let encryptor = Encryptor::with_user_passphrase(phrase.clone());
        let mut encrypted = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted)?;
        writer.write_all(secret.expose_secret().as_bytes())?;
        writer.finish()?;

        encrypted
    };

    Ok(Secret::new(token.to_vec()))
}

fn prompt_decrypt_passphrase() -> Result<Option<Secret<String>>, Error> {
    let phrase = dialoguer::Password::with_theme(tui_theme())
        .with_prompt("Decrypt with passphrase (optional)")
        .allow_empty_password(true)
        .report(false)
        .interact()?;

    if phrase.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Secret::new(phrase)))
    }
}

fn decrypt(secret: &Secret<String>, phrase: &Secret<String>) -> Result<Secret<String>, Error> {
    if phrase.expose_secret().is_empty() {
        return Ok(secret.clone());
    }

    // If a passphrase is provided we assume the secret is encoded bytes from encryption.
    let bytes = hex::decode(pad_hex(secret.expose_secret().clone()))?;
    if let Decryptor::Passphrase(decryptor) = Decryptor::new(bytes.as_slice())? {
        // Do the decryption
        let mut decrypted = vec![];
        let mut reader = decryptor.decrypt(phrase, None)?;
        reader.read_to_end(&mut decrypted)?;

        let token = String::from_utf8(decrypted).context("failed to parse utf-8")?;
        let secret = Secret::new(token);

        return Ok(secret);
    }

    bail!("failed to decrypt")
}

fn prompt_repository_url() -> Result<Option<Url>, Error> {
    eprint!("Repository URL: ");
    let url = get_trimmed_user_input().context("failed to read provided url")?;

    if url.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Url::parse(&url)?))
    }
}

fn get_trimmed_user_input() -> Result<String, Error> {
    std::io::stderr().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}

fn pad_hex(s: String) -> String {
    if s.len() % 2 == 1 {
        format!("0{}", s)
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_config_from_cli_with_token() {
        // User provides a token either from the CLI or credentials file (CLI here)
        let credentials_table = Item::Table(Table::new());

        let cli_repo = None; // no repo doesn't exclude pypi
        let cli_repo_url = None;
        let cli_username = None;
        let cli_token = Secret::new("token".to_string());

        let credentials =
            resolve_credentials(Some(&credentials_table), cli_username, Some(&cli_token));
        let repository =
            resolve_repository(Some(&credentials_table), cli_repo, cli_repo_url).unwrap();

        let config = PublishConfig {
            credentials,
            repository,
        };
        let config = config.resolve_with_defaults();

        // We are 'pypi' ready with defaults (and keyring ready)
        assert!(config_is_ready(&config));
        assert!(config_is_keyring_ready(&config));
    }

    #[test]
    fn test_config_from_cli_with_username() {
        let credentials_table = Item::Table(Table::new());

        let cli_repo = None; // no repo doesn't exclude pypi
        let cli_repo_url = None;
        let cli_username = "username";
        let cli_token = None;

        let credentials = resolve_credentials(
            Some(&credentials_table),
            Some(&cli_username.to_string()),
            cli_token,
        );
        let repository =
            resolve_repository(Some(&credentials_table), cli_repo, cli_repo_url).unwrap();

        let config = PublishConfig {
            credentials,
            repository,
        };
        let config = config.resolve_with_defaults();

        // We can fallback to keyring with a defaulted url
        assert!(config_is_ready(&config));
        assert!(config_is_keyring_ready(&config));
    }

    #[test]
    fn test_config_from_cli_with_url() {
        let credentials_table = Item::Table(Table::new());

        let cli_repo = None; // no repo doesn't exclude pypi
        let cli_repo_url = Url::parse("https://test.pypi.org/legacy/").unwrap();
        let cli_username = None;
        let cli_token = None;

        let credentials = resolve_credentials(Some(&credentials_table), cli_username, cli_token);
        let repository =
            resolve_repository(Some(&credentials_table), cli_repo, Some(cli_repo_url)).unwrap();

        let config = PublishConfig {
            credentials,
            repository,
        };
        let config = config.resolve_with_defaults();

        // We are ready because we can fallback to keyring
        assert!(config_is_ready(&config));
        assert!(config_is_keyring_ready(&config));
    }

    #[test]
    fn test_config_from_cli_with_username_token() {
        let credentials_table = Item::Table(Table::new());

        let cli_repo = None; // no repo doesn't exclude pypi
        let cli_repo_url = Url::parse("https://test.pypi.org/legacy/").unwrap();
        let cli_username = "username";
        let cli_token = Secret::new("token".to_string());

        let credentials = resolve_credentials(
            Some(&credentials_table),
            Some(&cli_username.to_string()),
            Some(&cli_token),
        );
        let repository =
            resolve_repository(Some(&credentials_table), cli_repo, Some(cli_repo_url)).unwrap();

        let config = PublishConfig {
            credentials,
            repository,
        };
        let config = config.resolve_with_defaults();

        // We are ready with username and password (token)
        assert!(config_is_ready(&config));
    }

    #[test]
    fn test_config_from_cli_with_keyring() {
        let credentials_table = Item::Table(Table::new());

        let cli_repo = None; // no repo doesn't exclude pypi
        let cli_repo_url = Url::parse("https://test.pypi.org/legacy/").unwrap();
        let cli_username = "username";
        let cli_token = Secret::new("token".to_string());

        let credentials = resolve_credentials(
            Some(&credentials_table),
            Some(&cli_username.to_string()),
            Some(&cli_token),
        );
        let repository =
            resolve_repository(Some(&credentials_table), cli_repo, Some(cli_repo_url)).unwrap();

        let config = PublishConfig {
            credentials,
            repository,
        };
        let config = config.resolve_with_defaults();

        // We are ready for keyring with username and url
        assert!(config_is_keyring_ready(&config));
    }

    #[test]
    fn test_repository_config_resolution_defaults() {
        // Resolve without credentials file
        let credentials_table = Item::Table(Table::new());

        let cli_repo = "pypi".to_string();
        let cli_repo_url = None;
        let cli_username = None;
        let cli_token = Secret::new("token".to_string());

        let credentials =
            resolve_credentials(Some(&credentials_table), cli_username, Some(&cli_token));
        let repository =
            resolve_repository(Some(&credentials_table), Some(cli_repo), cli_repo_url).unwrap();

        assert_eq!(credentials.username.unwrap(), DEFAULT_USERNAME);
        assert_eq!(
            credentials.password.unwrap().expose_secret(),
            cli_token.expose_secret()
        );
        assert_eq!(repository.url.unwrap().to_string(), DEFAULT_REPOSITORY_URL);
    }

    #[test]
    fn test_repository_config_file_only() {
        let mut credentials_table = Item::Table(Table::new());
        credentials_table["repository-url"] =
            Item::Value("https://test.pypi.org/".to_string().into());
        credentials_table["username"] = Item::Value("username".to_string().into());
        credentials_table["token"] = Item::Value("password".to_string().into());

        let credentials = resolve_credentials(Some(&credentials_table), None, None);
        let repository = resolve_repository(Some(&credentials_table), None, None).unwrap();

        let repository_url = Url::parse("https://test.pypi.org/").unwrap();
        let username = "username".to_string();
        let password = "password".to_string();

        assert_eq!(
            repository.url,
            Some(Url::parse("https://test.pypi.org/").unwrap())
        );
        assert_eq!(credentials.username, Some(username));
        assert_eq!(*credentials.password.unwrap().expose_secret(), password);
        assert_eq!(repository.url, Some(repository_url));
    }

    #[test]
    fn test_repository_config_cli_only() {
        // Resolve without credentials file
        let credentials_table = Item::Table(Table::new());

        let cli_repo = "pypi".to_string();
        let cli_repo_url = Url::parse("https://test.pypi.org/").unwrap();
        let cli_username = "username".to_string();
        let cli_token = Secret::new("token".to_string());

        let credentials = resolve_credentials(
            Some(&credentials_table),
            Some(&cli_username),
            Some(&cli_token),
        );
        let repository = resolve_repository(
            Some(&credentials_table),
            Some(cli_repo),
            Some(cli_repo_url.clone()),
        )
        .unwrap();

        assert_eq!(credentials.username, Some(cli_username));
        assert_eq!(
            credentials.password.unwrap().expose_secret(),
            cli_token.expose_secret()
        );
        assert_eq!(repository.url, Some(cli_repo_url));
    }

    #[test]
    fn test_repository_config_file_and_cli() {
        let mut credentials_table = Item::Table(Table::new());
        credentials_table["repository-url"] =
            Item::Value("https://test.pypi.org/".to_string().into());
        credentials_table["username"] = Item::Value("username".to_string().into());

        let cli_repo = "pypi".to_string();
        let cli_repo_url = None;
        let cli_username = None;
        let cli_token = Secret::new("token".to_string());

        let config = resolve_credentials(Some(&credentials_table), cli_username, Some(&cli_token));
        let repository =
            resolve_repository(Some(&credentials_table), Some(cli_repo), cli_repo_url).unwrap();

        assert_eq!(
            config.password.unwrap().expose_secret(),
            cli_token.expose_secret()
        );
        assert_eq!(
            repository.url.unwrap().to_string(),
            "https://test.pypi.org/".to_string()
        );
    }

    #[test]
    fn test_repository_config_keyring_fallback() {
        let credentials_table = Item::Table(Table::new());

        let cli_repo = "pypi".to_string();
        let cli_repo_url = Url::parse("https://test.pypi.org/").unwrap();
        let cli_username = None;
        let cli_token = None;

        let credentials = resolve_credentials(Some(&credentials_table), cli_username, cli_token);
        let repository = resolve_repository(
            Some(&credentials_table),
            Some(cli_repo),
            Some(cli_repo_url.clone()),
        )
        .unwrap();

        assert!(credentials.username.is_none());
        assert!(credentials.password.is_none());
        assert_eq!(repository.url.unwrap(), cli_repo_url);
    }

    #[test]
    fn test_save_rye_credentials_encrypt() {
        let tempdir = tempdir().unwrap();
        let temp_home = tempdir.path();
        let mut credentials_table = DocumentMut::new();

        let cli_repo = "pypi".to_string();

        // Set the environment variable for this specific test
        std::env::set_var("RYE_HOME", temp_home);
        crate::platform::init().unwrap();

        assert_eq!(
            std::env::var("RYE_HOME").map(PathBuf::from),
            Ok(temp_home.to_path_buf())
        );

        save_rye_credentials(
            &mut credentials_table,
            &Credentials {
                username: Some("username".to_string()),
                password: Some("password".to_string().into()),
            },
            &Repository {
                name: Some(cli_repo),
                url: None,
            },
            true,
            Some(&Secret::new("passphrase".to_string())),
        )
        .unwrap();

        let credentials = get_credentials().unwrap();
        let table = credentials.get("pypi");

        assert_eq!(
            table
                .and_then(|it| it.get("username"))
                .map(Item::to_string)
                .map(escape_string)
                .unwrap(),
            "username".to_string()
        );

        let token = table
            .and_then(|it| it.get("token").map(Item::to_string).map(escape_string))
            .unwrap();

        let password =
            decrypt(&Secret::new(token), &Secret::new("passphrase".to_string())).unwrap();

        assert_eq!(password.expose_secret(), "password");
    }
}
