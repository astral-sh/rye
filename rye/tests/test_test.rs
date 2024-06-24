use std::fs;

use insta::Settings;
use toml_edit::{value, Array};

use crate::common::{rye_cmd_snapshot, Space};

mod common;

const BASIC_TEST: &str = r#"
def test_okay():
    pass

def test_fail():
    1 / 0
"#;

// pytest for weird reasons has different formatting behavior on
// different platforms -.-
#[cfg(windows)]
const PYTEST_COLS: &str = "80";
#[cfg(unix)]
const PYTEST_COLS: &str = "79";

#[test]
fn test_basic_tool_behavior() {
    // fixes issues for rendering between platforms
    let mut settings = Settings::clone_current();
    settings.add_filter(r"(?m)^(platform )(.*?)( --)", "$1[PLATFORM]$3");
    settings.add_filter(r"(?m)\s+(\[\d+%\])\s*?$", " $1");
    let _guard = settings.bind_to_scope();

    let space = Space::new();
    space.init("foo");
    space.edit_toml("pyproject.toml", |doc| {
        let mut deps = Array::new();
        deps.push("pytest>=7.0.0");
        deps.push("colorama==0.4.6");
        let mut workspace_members = Array::new();
        workspace_members.push(".");
        workspace_members.push("child-dep");
        doc["tool"]["rye"]["dev-dependencies"] = value(deps);
        doc["tool"]["rye"]["workspace"]["members"] = value(workspace_members);
    });
    let status = space
        .rye_cmd()
        .arg("init")
        .arg("-q")
        .arg(space.project_path().join("child-dep"))
        .status()
        .unwrap();
    assert!(status.success());

    let root_tests = space.project_path().join("tests");
    fs::create_dir_all(&root_tests).unwrap();
    fs::write(root_tests.join("test_foo.py"), BASIC_TEST).unwrap();

    let child_tests = space.project_path().join("child-dep").join("tests");
    fs::create_dir_all(&child_tests).unwrap();
    fs::write(child_tests.join("test_child.py"), BASIC_TEST).unwrap();

    rye_cmd_snapshot!(space.rye_cmd().arg("test").env("COLUMNS", PYTEST_COLS), @r###"
    success: false
    exit_code: 1
    ----- stdout -----
    Initializing new virtualenv in [TEMP_PATH]/project/.venv
    Python version: cpython@3.12.3
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!
    Running tests for foo ([TEMP_PATH]/project)
    ============================= test session starts =============================
    platform [PLATFORM] -- Python 3.12.3, pytest-7.4.3, pluggy-1.3.0
    rootdir: [TEMP_PATH]/project
    collected 2 items

    tests/test_foo.py .F [100%]

    ================================== FAILURES ===================================
    __________________________________ test_fail __________________________________

        def test_fail():
    >       1 / 0
    E       ZeroDivisionError: division by zero

    tests/test_foo.py:6: ZeroDivisionError
    =========================== short test summary info ===========================
    FAILED tests/test_foo.py::test_fail - ZeroDivisionError: division by zero
    ========================= 1 failed, 1 passed in [EXECUTION_TIME] =========================

    ----- stderr -----
    Resolved 7 packages in [EXECUTION_TIME]
    Downloaded 7 packages in [EXECUTION_TIME]
    Installed 7 packages in [EXECUTION_TIME]
     + child-dep==0.1.0 (from file:[TEMP_PATH]/project/child-dep)
     + colorama==0.4.6
     + foo==0.1.0 (from file:[TEMP_PATH]/project)
     + iniconfig==2.0.0
     + packaging==23.2
     + pluggy==1.3.0
     + pytest==7.4.3
    "###);

    rye_cmd_snapshot!(space.rye_cmd().arg("test").arg("--all").env("COLUMNS", PYTEST_COLS), @r###"
    success: false
    exit_code: 1
    ----- stdout -----
    Running tests for child-dep ([TEMP_PATH]/project/child-dep)
    ============================= test session starts =============================
    platform [PLATFORM] -- Python 3.12.3, pytest-7.4.3, pluggy-1.3.0
    rootdir: [TEMP_PATH]/project/child-dep
    collected 2 items

    tests/test_child.py .F [100%]

    ================================== FAILURES ===================================
    __________________________________ test_fail __________________________________

        def test_fail():
    >       1 / 0
    E       ZeroDivisionError: division by zero

    tests/test_child.py:6: ZeroDivisionError
    =========================== short test summary info ===========================
    FAILED tests/test_child.py::test_fail - ZeroDivisionError: division by zero
    ========================= 1 failed, 1 passed in [EXECUTION_TIME] =========================

    Running tests for foo ([TEMP_PATH]/project)
    ============================= test session starts =============================
    platform [PLATFORM] -- Python 3.12.3, pytest-7.4.3, pluggy-1.3.0
    rootdir: [TEMP_PATH]/project
    collected 2 items

    tests/test_foo.py .F [100%]

    ================================== FAILURES ===================================
    __________________________________ test_fail __________________________________

        def test_fail():
    >       1 / 0
    E       ZeroDivisionError: division by zero

    tests/test_foo.py:6: ZeroDivisionError
    =========================== short test summary info ===========================
    FAILED tests/test_foo.py::test_fail - ZeroDivisionError: division by zero
    ========================= 1 failed, 1 passed in [EXECUTION_TIME] =========================

    ----- stderr -----
    "###);
}
