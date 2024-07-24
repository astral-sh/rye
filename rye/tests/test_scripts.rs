use toml_edit::value;

use crate::common::{rye_cmd_snapshot, Space};

mod common;

#[test]
fn test_basic_script() {
    let mut settings = insta::Settings::clone_current();
    settings.add_filter(r"(?m)(^py[a-z\d._]+$\r?\n)+", "[PYTHON SCRIPTS]\n");
    let _guard = settings.bind_to_scope();

    let space = Space::new();
    space.init("my-project");
    space.edit_toml("pyproject.toml", |doc| {
        doc["tool"]["rye"]["scripts"]["test-script"] = value("python -c 'print(\"Hello World\")'");
    });

    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("test-script"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Hello World

    ----- stderr -----
    Initializing new virtualenv in [TEMP_PATH]/project/.venv
    Python version: cpython@3.12.3
    "###);

    rye_cmd_snapshot!(space.rye_cmd().arg("run"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    [PYTHON SCRIPTS]
    test-script (python -c 'print("Hello World")')

    ----- stderr -----
    "###);
}
