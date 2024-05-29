use insta::Settings;

use crate::common::{rye_cmd_snapshot, Space};
mod common;

// This test is self-destructive, making other tests slow, ignore it by default.
#[test]
#[ignore]
fn test_self_uninstall() {
    let space = Space::new();
    let _guard = space.lock_rye_home();

    // install a global tool to ensure tools directory is created
    space
        .rye_cmd()
        .arg("install")
        .arg("pycowsay")
        .arg("-f")
        .status()
        .unwrap();

    assert!(space.rye_home().join("self").is_dir());
    assert!(space.rye_home().join("py").is_dir());
    assert!(space.rye_home().join("tools").is_dir());

    let status = space
        .rye_cmd()
        .arg("self")
        .arg("uninstall")
        .arg("--yes")
        .status()
        .unwrap();
    assert!(status.success());

    let may_left = &["env", "config.toml", "lock"];
    let leftovers: Vec<_> = space
        .rye_home()
        .read_dir()
        .unwrap()
        .filter(|x| {
            let x = x.as_ref().unwrap();
            !may_left.contains(&x.file_name().to_str().unwrap())
        })
        .collect();
    assert!(leftovers.is_empty(), "leftovers: {:?}", leftovers);
}

#[test]
fn test_version() {
    let space = Space::new();
    let _guard = space.lock_rye_home();

    let mut settings = Settings::clone_current();
    settings.add_filter(r"(?m)^(rye )\d+\.\d+\.\d+?$", "$1[VERSION]");
    settings.add_filter(r"(?m)^(commit: ).*?$", "$1[COMMIT]");
    settings.add_filter(r"(?m)^(platform: ).*?$", "$1[PLATFORM]");
    let _guard = settings.bind_to_scope();

    rye_cmd_snapshot!(space.rye_cmd().arg("--version"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    rye [VERSION]
    commit: [COMMIT]
    platform: [PLATFORM]
    self-python: cpython@3.12.3
    symlink support: true
    uv enabled: true

    ----- stderr -----
    "###);
}
