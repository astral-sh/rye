use insta::assert_snapshot;

use crate::common::{rye_cmd_snapshot, Space};

mod common;

#[test]
fn test_lint_and_format() {
    let space = Space::new();
    space.init("my-project");
    space.write(
        // `test.py` is used instead of `__init__.py` to make ruff consider it a fixable
        // issue instead of requiring user intervention.
        // ref: https://github.com/astral-sh/ruff/pull/11168
        "src/my_project/test.py",
        r#"import os

def hello():


    return "Hello World";
"#,
    );

    // start with lint
    rye_cmd_snapshot!(space.rye_cmd().arg("lint"), @r###"
    success: false
    exit_code: 1
    ----- stdout -----
    src/my_project/__init__.py:1:8: F401 [*] `os` imported but unused
    src/my_project/__init__.py:6:25: E703 [*] Statement ends with an unnecessary semicolon
    Found 2 errors.
    [*] 2 fixable with the `--fix` option.

    ----- stderr -----
    "###);
    rye_cmd_snapshot!(space.rye_cmd().arg("lint").arg("--fix"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Found 2 errors (2 fixed, 0 remaining).

    ----- stderr -----
    "###);
    assert_snapshot!(space.read_string("src/my_project/test.py"), @r###"

    def hello():


        return "Hello World"
    "###);

    // fmt next
    rye_cmd_snapshot!(space.rye_cmd().arg("fmt").arg("--check"), @r###"
    success: false
    exit_code: 1
    ----- stdout -----
    Would reformat: src/my_project/test.py
    1 file would be reformatted

    ----- stderr -----
    "###);
    rye_cmd_snapshot!(space.rye_cmd().arg("fmt"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    1 file reformatted

    ----- stderr -----
    "###);
    assert_snapshot!(space.read_string("src/my_project/test.py"), @r###"
    def hello():
        return "Hello World"
    "###);
}
