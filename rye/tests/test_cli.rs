use std::fs;

use toml_edit::value;

use crate::common::{rye_cmd_snapshot, Space};

mod common;

#[test]
fn test_dotenv() {
    let space = Space::new();
    space.init("my-project");
    space.edit_toml("pyproject.toml", |doc| {
        doc["tool"]["rye"]["scripts"]["hello"]["cmd"] =
            value("python -c \"import os; print(os.environ['MY_COOL_VAR'], os.environ['MY_COOL_OTHER_VAR'])\"");
        doc["tool"]["rye"]["scripts"]["hello"]["env-file"] = value(".other.env");
    });
    fs::write(space.project_path().join(".env"), "MY_COOL_VAR=42").unwrap();
    fs::write(
        space.project_path().join(".other.env"),
        "MY_COOL_OTHER_VAR=23",
    )
    .unwrap();
    rye_cmd_snapshot!(space.rye_cmd().arg("sync"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Initializing new virtualenv in [TEMP_PATH]/project/.venv
    Python version: cpython@3.12.3
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    Resolved 1 package in [EXECUTION_TIME]
    Downloaded 1 package in [EXECUTION_TIME]
    Installed 1 package in [EXECUTION_TIME]
     + my-project==0.1.0 (from file:[TEMP_PATH]/project)
    "###);
    rye_cmd_snapshot!(space.rye_cmd()
        .arg("--env-file=.env")
        .arg("run")
        .arg("hello"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    42 23

    ----- stderr -----
    "###);
}
