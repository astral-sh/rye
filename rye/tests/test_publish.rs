use crate::common::{rye_cmd_snapshot, Space};

mod common;

#[test]
fn test_publish_outside_project() {
    let space = Space::new();
    space.init("my-project");

    let status = space.rye_cmd().arg("build").status().unwrap();
    assert!(status.success());

    // Publish outside the project.
    // Since we provide a fake token, the failure is expected.
    rye_cmd_snapshot!(space
        .rye_cmd()
        .arg("publish")
        .arg("--yes")
        .arg("--token")
        .arg("fake-token")
        .arg("--quiet")
        .current_dir(space.project_path().parent().unwrap())
        .arg(space.project_path().join("dist").join("*")), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    error: failed to publish files
    "###);
}

#[test]
fn test_publish_outside_project_w_venv() {
    let venv_path = ".xyz";
    let space = Space::with_venv(venv_path);
    space.init("my-project");

    let status = space
        .rye_cmd()
        .arg("build")
        .arg("--venv")
        .arg(venv_path)
        .status()
        .unwrap();
    assert!(status.success());

    // Publish outside the project.
    // Since we provide a fake token, the failure is expected.
    rye_cmd_snapshot!(space
        .rye_cmd()
        .arg("publish")
        .arg("--venv")
        .arg(venv_path)
        .arg("--yes")
        .arg("--token")
        .arg("fake-token")
        .arg("--quiet")
        .current_dir(space.project_path().parent().unwrap())
        .arg(space.project_path().join("dist").join("*")), @r###"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    error: failed to publish files
    "###);
}
