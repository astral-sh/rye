use crate::common::{rye_cmd_snapshot, Space};

mod common;

#[test]
fn test_empty_sync() {
    let space = Space::new();
    space.init("my-project");
    rye_cmd_snapshot!(space.rye_cmd().arg("sync"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Initializing new virtualenv in [TEMP_PATH]/project/.venv
    Python version: cpython@3.12.1
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    warning: Requirements file [TEMP_FILE] does not contain any dependencies
    Built 1 editable in [EXECUTION_TIME]
    Resolved 1 package in [EXECUTION_TIME]
    warning: Requirements file [TEMP_FILE] does not contain any dependencies
    Built 1 editable in [EXECUTION_TIME]
    Resolved 1 package in [EXECUTION_TIME]
    Built 1 editable in [EXECUTION_TIME]
    Installed 1 package in [EXECUTION_TIME]
     + my-project==0.1.0 (from file:[TEMP_PATH]/project)
    "###);
}

#[test]
fn test_add_and_sync() {
    let space = Space::new();
    space.init("my-project");
    // add colorama to ensure we have this as a dependency on all platforms
    rye_cmd_snapshot!(space.rye_cmd().arg("add").arg("flask==3.0.0").arg("colorama"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Initializing new virtualenv in [TEMP_PATH]/project/.venv
    Python version: cpython@3.12.1
    Added colorama>=0.4.6 as regular dependency
    Added flask>=3.0.0 as regular dependency

    ----- stderr -----
    "###);
    rye_cmd_snapshot!(space.rye_cmd().arg("sync"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Reusing already existing virtualenv
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    warning: Requirements file [TEMP_FILE] does not contain any dependencies
    Built 1 editable in [EXECUTION_TIME]
    Resolved 9 packages in [EXECUTION_TIME]
    warning: Requirements file [TEMP_FILE] does not contain any dependencies
    Built 1 editable in [EXECUTION_TIME]
    Resolved 9 packages in [EXECUTION_TIME]
    Built 1 editable in [EXECUTION_TIME]
    Resolved 8 packages in [EXECUTION_TIME]
    Downloaded 8 packages in [EXECUTION_TIME]
    Installed 9 packages in [EXECUTION_TIME]
     + blinker==1.7.0
     + click==8.1.7
     + colorama==0.4.6
     + flask==3.0.0
     + itsdangerous==2.1.2
     + jinja2==3.1.2
     + markupsafe==2.1.3
     + my-project==0.1.0 (from file:[TEMP_PATH]/project)
     + werkzeug==3.0.1
    "###);
}
