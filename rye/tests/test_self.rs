use crate::common::Space;
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
