use crate::common::{rye_cmd_snapshot, Space};

mod common;

#[test]
fn test_fetch() {
    let space = Space::new();
    // Use a version not in use by other tests and will be supported for a long time.
    let version = "cpython@3.12.1";

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
    Downloading cpython@3.12.1
    Checking checksum
    Unpacking
    Downloaded cpython@3.12.1

    ----- stderr -----
    "###);
}
