use crate::common::{rye_cmd_snapshot, Space};
use std::fs;
use toml_edit::{table, value, Array};

mod common;

#[test]
fn test_run_list() {
    let space = Space::new();
    space.init("my-project");

    let status = space
        .rye_cmd()
        .arg("add")
        .arg("Flask==3.0.0")
        .arg("--sync")
        .status()
        .unwrap();
    assert!(status.success());

    space.edit_toml("pyproject.toml", |doc| {
        let mut scripts = table();
        scripts["hello"] = value("echo hello");
        doc["tool"]["rye"]["scripts"] = scripts;
    });

    #[cfg(not(windows))]
    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("--list"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    flask
    hello (echo hello)
    python
    python3
    python3.12

    ----- stderr -----
    "###);
    #[cfg(windows)]
    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("--list"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    flask
    hello (echo hello)
    pydoc
    python
    python3
    python3.12
    pythonw

    ----- stderr -----
    "###);
}

#[test]
fn test_basic_run() {
    let space = Space::new();
    space.init("my-project");

    // Run a virtualenv script
    let status = space
        .rye_cmd()
        .arg("add")
        .arg("Flask==3.0.0")
        .arg("--sync")
        .status()
        .unwrap();
    assert!(status.success());
    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("flask").arg("--version"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Python 3.12.2
    Flask 3.0.0
    Werkzeug 3.0.1

    ----- stderr -----
    "###);

    // Run a non-existing script
    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("not_exist_script"), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    error: invalid or unknown script 'not_exist_script'
    "###);

    let init_script = space
        .project_path()
        .join("src")
        .join("my_project")
        .join("__init__.py");
    fs::write(
        &init_script,
        "def hello():\n    print('Hello from my-project!')\n    return 0",
    )
    .unwrap();

    let env_file = space.project_path().join("env_file");
    fs::write(&env_file, r#"HELLO="Hello from env_file!""#).unwrap();

    // Run Rye scripts
    space.edit_toml("pyproject.toml", |doc| {
        let mut scripts = table();
        // A simple string command
        scripts["script_1"] = value(r#"python -c 'print("Hello from script_1!")'"#);
        // A simple command using `cmd` key
        scripts["script_2"]["cmd"] = value(r#"python -c 'print("Hello from script_2!")'"#);
        // A `call` script
        scripts["script_3"]["call"] = value("my_project:hello");
        // A failing script
        scripts["script_4"]["cmd"] = value(r#"python -c 'import sys; sys.exit(1)'"#);
        // A `chain` script
        scripts["script_5"]["chain"] =
            value(Array::from_iter(["script_1", "script_2", "script_3"]));
        // A failing `chain` script
        scripts["script_6"]["chain"] = value(Array::from_iter([
            "script_1", "script_2", "script_3", "script_4", "script_3",
        ]));
        // A script with environment variables
        scripts["script_7"]["cmd"] = value(r#"python -c 'import os; print(os.getenv("HELLO"))'"#);
        scripts["script_7"]["env"]["HELLO"] = value("Hello from script_7!");
        // A script with an env-file
        scripts["script_8"]["cmd"] = value(r#"python -c 'import os; print(os.getenv("HELLO"))'"#);
        scripts["script_8"]["env-file"] = value(env_file.to_string_lossy().into_owned());

        doc["tool"]["rye"]["scripts"] = scripts;
    });

    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("script_1"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Hello from script_1!

    ----- stderr -----
    "###);
    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("script_2"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Hello from script_2!

    ----- stderr -----
    "###);
    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("script_3"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Hello from my-project!

    ----- stderr -----
    "###);
    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("script_4"), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    "###);
    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("script_5"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Hello from script_1!
    Hello from script_2!
    Hello from my-project!

    ----- stderr -----
    "###);
    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("script_6"), @r###"
    success: false
    exit_code: 1
    ----- stdout -----
    Hello from script_1!
    Hello from script_2!
    Hello from my-project!

    ----- stderr -----
    error: script failed with exit status: 1
    "###);
    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("script_7"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Hello from script_7!

    ----- stderr -----
    "###);
    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("script_8"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Hello from env_file!

    ----- stderr -----
    "###);
}

#[test]
fn test_run_name_collision() {
    let space = Space::new();
    space.init("my-project");

    let status = space
        .rye_cmd()
        .arg("add")
        .arg("Flask==3.0.0")
        .arg("--sync")
        .status()
        .unwrap();
    assert!(status.success());

    space.edit_toml("pyproject.toml", |doc| {
        doc["tool"]["rye"]["scripts"] = table();
        doc["tool"]["rye"]["scripts"]["flask"] =
            value(r#"python -c 'print("flask from rye script")'"#);
    });
    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("flask").arg("--version"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Python 3.12.2
    Flask 3.0.0
    Werkzeug 3.0.1

    ----- stderr -----
    "###);
}
