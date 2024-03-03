use std::fs;

use insta::{assert_snapshot, Settings};

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
    Python version: cpython@3.12.2
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    Built 1 editable in [EXECUTION_TIME]
    Installed 1 package in [EXECUTION_TIME]
     + my-project==0.1.0 (from file:[TEMP_PATH]/project)
    "###);

    // is the prompt set?
    #[cfg(unix)]
    {
        let script = space.venv_path().join("bin/activate");
        let contents = fs::read_to_string(script).unwrap();
        assert!(contents.contains("VIRTUAL_ENV_PROMPT=\"my-project\""));
    }
    #[cfg(windows)]
    {
        let script = space.venv_path().join("Scripts/activate.bat");
        let contents = fs::read_to_string(script).unwrap();
        assert!(contents.contains("@set \"VIRTUAL_ENV_PROMPT=my-project\""));
    }
}

#[test]
fn test_add_and_sync_no_auto_sync() {
    let space = Space::new();
    space.init("my-project");

    // add colorama to ensure we have this as a dependency on all platforms
    rye_cmd_snapshot!(space.rye_cmd().arg("add").arg("flask==3.0.0").arg("colorama").arg("--no-sync"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Initializing new virtualenv in [TEMP_PATH]/project/.venv
    Python version: cpython@3.12.2
    Added flask>=3.0.0 as regular dependency
    Added colorama>=0.4.6 as regular dependency

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
fn test_add_autosync() {
    let space = Space::new();
    space.init("my-project");
    // add colorama to ensure we have this as a dependency on all platforms
    rye_cmd_snapshot!(space.rye_cmd().arg("add").arg("flask==3.0.0").arg("colorama"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Initializing new virtualenv in [TEMP_PATH]/project/.venv
    Python version: cpython@3.12.2
    Added flask>=3.0.0 as regular dependency
    Added colorama>=0.4.6 as regular dependency
    Reusing already existing virtualenv
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
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
fn test_autosync_remember() {
    // remove the dependency source markers since they are instable between platforms
    let mut settings = Settings::clone_current();
    settings.add_filter(r"(?m)^\s+# via .*\r?\n", "");
    let _guard = settings.bind_to_scope();

    let space = Space::new();
    space.init("my-project");
    rye_cmd_snapshot!(space.rye_cmd().arg("sync").arg("--with-sources").arg("--all-features"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Initializing new virtualenv in [TEMP_PATH]/project/.venv
    Python version: cpython@3.12.2
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    Built 1 editable in [EXECUTION_TIME]
    Installed 1 package in [EXECUTION_TIME]
     + my-project==0.1.0 (from file:[TEMP_PATH]/project)
    "###);

    rye_cmd_snapshot!(space.rye_cmd()
        .arg("add").arg("--optional=web").arg("flask==3.0.0").arg("colorama"),
        @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Added flask>=3.0.0 as optional (web) dependency
    Added colorama>=0.4.6 as optional (web) dependency
    Reusing already existing virtualenv
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    Built 1 editable in [EXECUTION_TIME]
    Resolved 8 packages in [EXECUTION_TIME]
    Downloaded 8 packages in [EXECUTION_TIME]
    Uninstalled 1 package in [EXECUTION_TIME]
    Installed 9 packages in [EXECUTION_TIME]
     + blinker==1.7.0
     + click==8.1.7
     + colorama==0.4.6
     + flask==3.0.0
     + itsdangerous==2.1.2
     + jinja2==3.1.2
     + markupsafe==2.1.3
     - my-project==0.1.0 (from file:[TEMP_PATH]/project)
     + my-project==0.1.0 (from file:[TEMP_PATH]/project)
     + werkzeug==3.0.1
    "###);
    assert_snapshot!(std::fs::read_to_string(space.project_path().join("requirements.lock")).unwrap(), @r###"
    # generated by rye
    # use `rye lock` or `rye sync` to update this lockfile
    #
    # last locked with the following flags:
    #   pre: false
    #   features: []
    #   all-features: true
    #   with-sources: true

    --index-url https://pypi.org/simple/

    -e file:.
    blinker==1.7.0
    click==8.1.7
    colorama==0.4.6
    flask==3.0.0
    itsdangerous==2.1.2
    jinja2==3.1.2
    markupsafe==2.1.3
    werkzeug==3.0.1
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
    Audited 9 packages in [EXECUTION_TIME]
    "###);
    assert_snapshot!(std::fs::read_to_string(space.project_path().join("requirements.lock")).unwrap(), @r###"
    # generated by rye
    # use `rye lock` or `rye sync` to update this lockfile
    #
    # last locked with the following flags:
    #   pre: false
    #   features: []
    #   all-features: true
    #   with-sources: true

    --index-url https://pypi.org/simple/

    -e file:.
    blinker==1.7.0
    click==8.1.7
    colorama==0.4.6
    flask==3.0.0
    itsdangerous==2.1.2
    jinja2==3.1.2
    markupsafe==2.1.3
    werkzeug==3.0.1
    "###);
}
