use crate::common::{rye_cmd_snapshot, Space};

mod common;

#[test]
fn test_version_show() {
    let space = Space::new();
    space.init("my-project");
    rye_cmd_snapshot!(space.rye_cmd().arg("version"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    0.1.0

    ----- stderr -----
    "###);
}

#[test]
fn test_version_bump() {
    let space = Space::new();
    space.init("my-project");
    rye_cmd_snapshot!(space.rye_cmd().arg("version").arg("--bump").arg("patch"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    version bumped to 0.1.1

    ----- stderr -----
    "###);

    rye_cmd_snapshot!(space.rye_cmd().arg("version").arg("--bump").arg("minor"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    version bumped to 0.2.0

    ----- stderr -----
    "###);

    rye_cmd_snapshot!(space.rye_cmd().arg("version").arg("--bump").arg("major"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    version bumped to 1.0.0

    ----- stderr -----
    "###);
}
