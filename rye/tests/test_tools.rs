use std::env::consts::EXE_EXTENSION;
use std::fs;

use crate::common::{rye_cmd_snapshot, Space};

mod common;

#[test]
fn test_basic_tool_behavior() {
    let space = Space::new();

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
    Resolved 1 package in [EXECUTION_TIME]
    Downloaded 1 package in [EXECUTION_TIME]
    Installed 1 package in [EXECUTION_TIME]
     + pycowsay==0.0.0.2
    "###);

    rye_cmd_snapshot!(
        space.rye_cmd()
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
            .arg("tools")
            .arg("list")
            .arg("--include-version"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    pycowsay 0.0.0.2 (cpython@3.11.7)

    ----- stderr -----
    "###);

    rye_cmd_snapshot!(
        space.rye_cmd()
            .arg("toolchain")
            .arg("remove")
            .arg("cpython@3.11.7"), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    error: toolchain cpython@3.11.7 is still in use by tool pycowsay
    "###);

    rye_cmd_snapshot!(
        space.rye_cmd()
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
