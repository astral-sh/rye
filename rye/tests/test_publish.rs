use insta_cmd::Command;

use crate::common::{rye_cmd_snapshot, Space};

mod common;

#[test]
fn test_publish() {
    let space = Space::new();
    space.init("my-project");

    rye_cmd_snapshot!(with_skip_save(space.rye_cmd().arg("publish")), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    error: relative URL without a base
    "###);
}

#[test]
fn test_publish_yes() {
    let space = Space::new();
    space.init("my-project");

    rye_cmd_snapshot!(with_skip_save(space.rye_cmd().arg("publish").arg("-y")), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    error: relative URL without a base
    "###);
}

#[test]
fn test_publish_from_credentials_missing_repo() {
    let space = Space::new();
    space.init("my-project");

    rye_cmd_snapshot!(with_skip_save(space.rye_cmd().arg("publish").arg("-r").arg("missing")), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Access token: Repository URL: error: failed to resolve configuration for repository 'missing'
    "###);
}

#[test]
fn test_publish_from_credentials_missing_repo_yes() {
    let space = Space::new();
    space.init("my-project");

    rye_cmd_snapshot!(with_skip_save(space.rye_cmd().arg("publish").arg("-r").arg("missing").arg("-y")), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    error: failed to resolve configuration for repository 'missing'
    "###);
}

#[test]
fn test_publish_from_credentials_found_repo_with_username() {
    let space = Space::new();
    space.init("my-project");

    rye_cmd_snapshot!(with_skip_save(space.rye_cmd().arg("publish").arg("-r").arg("found-username")), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Access token: Repository URL: error: failed to resolve configuration for repository 'found-username'
    "###);
}

#[test]
fn test_publish_from_credentials_found_repo_with_token() {
    let space = Space::new();
    space.init("my-project");

    rye_cmd_snapshot!(with_skip_save(space.rye_cmd().arg("publish").arg("-r").arg("found-token")), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Repository URL: error: failed to resolve configuration for repository 'found-token'
    "###);
}

#[test]
fn test_publish_from_credentials_found_repo_with_username_token() {
    let space = Space::new();
    space.init("my-project");

    rye_cmd_snapshot!(with_skip_save(space.rye_cmd().arg("publish").arg("-r").arg("found-username-token")), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Repository URL: error: failed to resolve configuration for repository 'found-username-token'
    "###);
}

#[test]
fn test_publish_from_credentials_found_repo_with_username_yes() {
    let space = Space::new();
    space.init("my-project");

    rye_cmd_snapshot!(with_skip_save(space.rye_cmd().arg("publish").arg("-r").arg("found-username")).arg("-y"), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    error: failed to resolve configuration for repository 'found-username'
    "###);
}

#[test]
fn test_publish_from_credentials_found_repo_with_token_yes() {
    let space = Space::new();
    space.init("my-project");

    rye_cmd_snapshot!(with_skip_save(space.rye_cmd().arg("publish").arg("-r").arg("found-token")).arg("-y"), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    error: failed to resolve configuration for repository 'found-token'
    "###);
}

#[test]
fn test_publish_from_credentials_found_repo_with_username_token_yes() {
    let space = Space::new();
    space.init("my-project");

    rye_cmd_snapshot!(with_skip_save(space.rye_cmd().arg("publish").arg("-r").arg("found-username-token")).arg("-y"), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    error: failed to resolve configuration for repository 'found-username-token'
    "###);
}

fn with_skip_save(cmd: &mut Command) -> &mut Command {
    cmd.arg("--skip-save-credentials")
}
