use std::fs;

use insta::assert_debug_snapshot;
use toml_edit::{value, Array};

use crate::common::{rye_cmd_snapshot, Space};

mod common;

const BASIC_TEST: &str = r#"
def test_okay():
    pass

def test_fail():
    1 / 0
"#;

#[test]
fn test_basic_tool_behavior() {
    let space = Space::new();
    space.init("foo");
    space.edit_toml("pyproject.toml", |doc| {
        let mut deps = Array::new();
        deps.push("pytest>=7.0.0");
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

    rye_cmd_snapshot!(space.rye_cmd().arg("test"), @r###"
    success: false
    exit_code: 1
    ----- stdout -----
    Initializing new virtualenv in [TEMP_PATH]/project/.venv
    Python version: cpython@3.12.2
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!
    Running tests for foo ([TEMP_PATH]/project)
    ============================= test session starts =============================
    platform win32 -- Python 3.12.2, pytest-7.4.3, pluggy-1.3.0
    rootdir: [TEMP_PATH]/project
    collected 2 items

    tests/test_foo.py .F                                                     [100%]

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
    Built 2 editables in [EXECUTION_TIME]
    Resolved 3 packages in [EXECUTION_TIME]
    Downloaded 3 packages in [EXECUTION_TIME]
    Installed 7 packages in [EXECUTION_TIME]
     + child-dep==0.1.0 (from file:[TEMP_PATH]/project/child-dep)
     + colorama==0.4.6
     + foo==0.1.0 (from file:[TEMP_PATH]/project)
     + iniconfig==2.0.0
     + packaging==23.2
     + pluggy==1.3.0
     + pytest==7.4.3
    "###);

    rye_cmd_snapshot!(space.rye_cmd().arg("test").arg("--all"), @r###"
    success: false
    exit_code: 1
    ----- stdout -----
    Running tests for child-dep ([TEMP_PATH]/project/child-dep)
    ============================= test session starts =============================
    platform win32 -- Python 3.12.2, pytest-7.4.3, pluggy-1.3.0
    rootdir: [TEMP_PATH]/project/child-dep
    collected 2 items

    tests/test_child.py .F                                                   [100%]

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
    platform win32 -- Python 3.12.2, pytest-7.4.3, pluggy-1.3.0
    rootdir: [TEMP_PATH]/project
    collected 2 items

    tests/test_foo.py .F                                                     [100%]

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
