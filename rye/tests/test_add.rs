use toml_edit::{value, ArrayOfTables, Table};

use crate::common::{rye_cmd_snapshot, Space};

mod common;

#[test]
fn test_add_flask() {
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

#[test]
fn test_add_from_find_links() {
    let space = Space::new();
    space.init("my-project");
    space.edit_toml("pyproject.toml", |doc| {
        let mut source = Table::new();
        source["name"] = value("extra");
        source["type"] = value("find-links");
        source["url"] = value("https://download.pytorch.org/whl/torch_stable.html");
        let mut sources = ArrayOfTables::new();
        sources.push(source);
        doc["tool"]["rye"]["sources"] = value(sources.into_array());
    });

    rye_cmd_snapshot!(space.rye_cmd().arg("add").arg("tqdm").arg("colorama"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Initializing new virtualenv in [TEMP_PATH]/project/.venv
    Python version: cpython@3.12.1
    Added colorama>=0.4.6 as regular dependency
    Added tqdm>=4.66.1 as regular dependency
    Reusing already existing virtualenv
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    warning: Requirements file [TEMP_FILE] does not contain any dependencies
    Built 1 editable in [EXECUTION_TIME]
    Resolved 3 packages in [EXECUTION_TIME]
    warning: Requirements file [TEMP_FILE] does not contain any dependencies
    Built 1 editable in [EXECUTION_TIME]
    Resolved 3 packages in [EXECUTION_TIME]
    Built 1 editable in [EXECUTION_TIME]
    Resolved 2 packages in [EXECUTION_TIME]
    Downloaded 2 packages in [EXECUTION_TIME]
    Installed 3 packages in [EXECUTION_TIME]
     + colorama==0.4.6
     + my-project==0.1.0 (from file:[TEMP_PATH]/project)
     + tqdm==4.66.1
    "###);
}
