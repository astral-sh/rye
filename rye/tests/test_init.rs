use crate::common::{get_bin, rye_cmd_snapshot, Space};

mod common;

// Test that init --lib works
#[test]
fn test_init_lib() {
    let space = Space::new();
    space
        .cmd(get_bin())
        .arg("init")
        .arg("--name")
        .arg("my-project")
        .arg("-q")
        .arg("--lib")
        .current_dir(space.project_path())
        .status()
        .expect("initialization successful");

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
    Built 1 editable in [EXECUTION_TIME]
    Installed 1 package in [EXECUTION_TIME]
     + my-project==0.1.0 (from file:[TEMP_PATH]/project)
    "###);

    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("python").arg("-c").arg("import my_project; print(my_project.hello())"), @r###"
        success: true
        exit_code: 0
        ----- stdout -----
        Hello from my-project!

        ----- stderr -----
    "###);

    assert!(
        space.read_toml("pyproject.toml")["project"]
            .get("scripts")
            .is_none(),
        "[project.scripts] should not be present"
    )
}

// The default is the same as --lib
#[test]
fn test_init_default() {
    let space = Space::new();
    space
        .cmd(get_bin())
        .arg("init")
        .arg("--name")
        .arg("my-project")
        .arg("-q")
        .current_dir(space.project_path())
        .status()
        .expect("initialization successful");

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
    Built 1 editable in [EXECUTION_TIME]
    Installed 1 package in [EXECUTION_TIME]
     + my-project==0.1.0 (from file:[TEMP_PATH]/project)
    "###);

    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("python").arg("-c").arg("import my_project; print(my_project.hello())"), @r###"
        success: true
        exit_code: 0
        ----- stdout -----
        Hello from my-project!

        ----- stderr -----
    "###);

    assert!(
        space.read_toml("pyproject.toml")["project"]
            .get("scripts")
            .is_none(),
        "[project.scripts] should not be present"
    )
}

// Test that init --script works
#[test]
fn test_init_script() {
    let space = Space::new();
    space
        .cmd(get_bin())
        .arg("init")
        .arg("--name")
        .arg("my-project")
        .arg("-q")
        .arg("--script")
        .current_dir(space.project_path())
        .status()
        .expect("initialization successful");

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
    Built 1 editable in [EXECUTION_TIME]
    Installed 1 package in [EXECUTION_TIME]
     + my-project==0.1.0 (from file:[TEMP_PATH]/project)
    "###);

    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("my-project"), @r###"
        success: true
        exit_code: 0
        ----- stdout -----
        Hello from my-project!

        ----- stderr -----
    "###);

    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("python").arg("-mmy_project"), @r###"
        success: true
        exit_code: 0
        ----- stdout -----
        Hello from my-project!

        ----- stderr -----
    "###);
}

// Test that init --script and --lib are incompatible.
#[test]
fn test_init_lib_and_script_incompatible() {
    let space = Space::new();
    rye_cmd_snapshot!(space.cmd(get_bin()).arg("init").arg("--name").arg("my-project").arg("--script").arg("--lib").current_dir(space.project_path()), @r###"
        success: false
        exit_code: 2
        ----- stdout -----

        ----- stderr -----
        error: an argument cannot be used with one or more of the other specified arguments
    "###);
}

// Test init -r requirements.txt; that importing a requirements file works
#[test]
fn test_init_r_requirements() {
    let space = Space::new();

    let dependencies = vec![
        String::from("package_a==1.17.173"),
        String::from("package_b @ file:///${PROJECT_ROOT}/nested/b"),
        String::from("package_c @ https://${USERNAME}:${PASSWORD}@example.com/"),
        String::from("package_d @ git+https://githost.example.com/user/repo.git@tag"),
    ];

    space.write("requirements.txt", dependencies.join("\n") + "\n");

    space
        .cmd(get_bin())
        .arg("init")
        .arg("--name")
        .arg("my-project")
        .arg("-q")
        .arg("-r")
        .arg("requirements.txt")
        .current_dir(space.project_path())
        .status()
        .expect("initialization successful");

    assert_eq!(
        space.read_toml("pyproject.toml")["project"]
            .get("dependencies")
            .and_then(|v| v.as_array())
            .map(common::toml_array_as_string_array)
            .as_ref(),
        Some(&dependencies),
        "dependencies should match after import"
    )
}
