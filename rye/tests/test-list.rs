use crate::common::{rye_cmd_snapshot, Space};
use toml_edit::value;

mod common;

#[test]
fn test_basic_list() {
    let space = Space::new();
    space.init("my-project");

    space
        .rye_cmd()
        .arg("add")
        .arg("jinja2")
        .status()
        .expect("ok");

    rye_cmd_snapshot!(
        space.rye_cmd().arg("list"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    jinja2==3.1.2
    markupsafe==2.1.3
    -e file:[TEMP_PATH]/project

    ----- stderr -----
    "###);
}

#[test]
fn test_list_not_rye_managed() {
    let space = Space::new();
    space.init("my-project");

    space.edit_toml("pyproject.toml", |doc| {
        doc["tool"]["rye"]["managed"] = value(false);
    });

    space
        .rye_cmd()
        .arg("add")
        .arg("jinja2")
        .status()
        .expect("Add package failed");

    rye_cmd_snapshot!(
        space.rye_cmd().arg("list"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    jinja2==3.1.2
    markupsafe==2.1.3
    -e file:[TEMP_PATH]/project

    ----- stderr -----
    "###);
}

#[test]
fn test_list_never_overwrite() {
    let space = Space::new();
    space.init("my-project");

    space.rye_cmd().arg("sync").status().expect("Sync failed");

    let venv_marker = space.read_string(".venv/rye-venv.json");
    assert!(
        venv_marker.contains("@3.12"),
        "asserting contents of venv marker: {}",
        venv_marker
    );

    // Pick different python version
    space
        .rye_cmd()
        .arg("pin")
        .arg("3.11")
        .status()
        .expect("Sync failed");

    // List keeps the existing virtualenv unchanged

    rye_cmd_snapshot!(
        space.rye_cmd().arg("list"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    -e file:[TEMP_PATH]/project

    ----- stderr -----
    "###);

    let venv_marker = space.read_string(".venv/rye-venv.json");
    assert!(
        venv_marker.contains("@3.12"),
        "asserting contents of venv marker: {}",
        venv_marker
    );
}
