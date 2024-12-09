use std::env::consts::EXE_EXTENSION;
use std::fs;
use tempfile::TempDir;

use crate::common::{rye_cmd_snapshot, Space};

mod common;

#[test]
fn test_basic_tool_behavior() {
    let space = Space::new();

    // Cache alongside the home directory, so that the cache lives alongside the tools directory.
    let cache_dir = TempDir::new_in(space.rye_home()).unwrap();

    // in case we left things behind from last run.
    fs::remove_dir_all(space.rye_home().join("tools")).ok();
    fs::remove_file(
        space
            .rye_home()
            .join("shims")
            .join("pycowsay")
            .with_extension(EXE_EXTENSION),
    )
    .ok();

    rye_cmd_snapshot!(
        space.rye_cmd()
            .env("UV_CACHE_DIR", cache_dir.path())
            .arg("tools")
            .arg("install")
            .arg("pycowsay")
            .arg("-p")
            .arg("cpython@3.11"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    Installed scripts:
      - pycowsay

    ----- stderr -----
    Using Python 3.11.11 environment at: [RYE_HOME]/tools/pycowsay
    Resolved 1 package in [EXECUTION_TIME]
    Prepared 1 package in [EXECUTION_TIME]
    Installed 1 package in [EXECUTION_TIME]
     + pycowsay==0.0.0.2
    "###);

    rye_cmd_snapshot!(
        space.rye_cmd()
            .env("UV_CACHE_DIR", cache_dir.path())
            .arg("tools")
            .arg("list"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    pycowsay

    ----- stderr -----
    "###);

    rye_cmd_snapshot!(
        space.rye_cmd()
            .env("UV_CACHE_DIR", cache_dir.path())
            .arg("tools")
            .arg("list")
            .arg("--include-version"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    pycowsay 0.0.0.2 (cpython@3.11.11)

    ----- stderr -----
    "###);

    rye_cmd_snapshot!(
        space.rye_cmd()
            .env("UV_CACHE_DIR", cache_dir.path())
            .arg("toolchain")
            .arg("remove")
            .arg("cpython@3.11.11"), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    error: toolchain cpython@3.11.11 is still in use by tool pycowsay
    "###);

    rye_cmd_snapshot!(
        space.rye_cmd()
            .env("UV_CACHE_DIR", cache_dir.path())
            .arg("tools")
            .arg("uninstall")
            .arg("pycowsay"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Uninstalled pycowsay

    ----- stderr -----
    "###);

    assert!(!space.rye_home().join("tools").join("pycowsay").is_dir());
}
