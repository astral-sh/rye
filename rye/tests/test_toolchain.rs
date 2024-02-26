use crate::common::{rye_cmd_snapshot, Space};

mod common;

#[test]
fn test_fetch() {
    let space = Space::new();
    let version = "cpython@3.12.2";

    // Make sure the version is installed.
    let status = space.rye_cmd().arg("fetch").arg(version).status().unwrap();
    assert!(status.success());

    // Fetching the same version again should be a no-op.
    rye_cmd_snapshot!(space.rye_cmd().arg("fetch").arg(version).arg("--verbose"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Python version already downloaded. Skipping.

    ----- stderr -----
    "###);

    // Fetching the same version again with --force should re-download it.
    rye_cmd_snapshot!(space.rye_cmd().arg("fetch").arg(version).arg("--force"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Removing the existing Python version
    Downloading cpython@3.12.2
    Checking checksum
    Unpacking
    Downloaded cpython@3.12.2

    ----- stderr -----
    "###);

    rye_cmd_snapshot!(space.rye_cmd().arg("toolchain").arg("list"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython@3.12.2 ([RYE_HOME]/py/cpython@3.12.2/install/bin/python3)
    cpython@3.12.1 ([RYE_HOME]/py/cpython@3.12.1/install/bin/python3)
    cpython@3.11.8 ([RYE_HOME]/py/cpython@3.11.8/install/bin/python3)
    cpython@3.8.17 ([RYE_HOME]/py/cpython@3.8.17/install/bin/python3)

    ----- stderr -----
    "###);
}
