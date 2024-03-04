use insta::assert_snapshot;

use crate::common::{rye_cmd_snapshot, Space};

mod common;

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "musl"))]
fn test_bootstrap_linux_musl_defined() {
    let space = Space::new();
    space.init_script("my-project");

    rye_cmd_snapshot!(
        space.rye_cmd()
        .arg("self")
        .arg("install")
        .arg("--toolchain-version=cpython-x86_64-linux-musl@3.12.2")
        .arg("--yes"),
        @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Welcome to Rye!

    This installer will install rye to [RYE_HOME]
    This path can be changed by exporting the RYE_HOME environment variable.

    Details:
      Rye Version: 0.28.0
      Platform: linux (x86_64)
      Internal Toolchain Version: cpython-x86_64-linux-musl@3.12.2

    Installed binary to [RYE_HOME]/shims/rye
    Updated self-python installation at [RYE_HOME]/self

    The rye directory [RYE_HOME]/shims was not detected on PATH.
    It is highly recommended that you add it.
    Added to PATH.
    note: for this to take effect you will need to restart your shell or run this manually:

        source "[RYE_HOME]/env"

    For more information read https://rye-up.com/guide/installation/

    All done!

    ----- stderr -----
    "###
    );

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

    rye_cmd_snapshot!(space.rye_cmd().arg("run"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    my-project
    python
    python3
    python3.12

    "###);

    space.write(
        "src/my_project/__init__.py",
        r#"
import sysconfig
def main():
    cc = sysconfig.get_config_var('CC')
    linkcc = sysconfig.get_config_var('LINKCC')
    if 'musl' in cc and 'musl' in linkcc:
        return 0
    else:
        return 1
"#,
    );

    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("my-project"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    "###);
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "musl"))]
fn test_bootstrap_linux_musl() {
    let space = Space::new();
    space.init_script("my-project");

    rye_cmd_snapshot!(
        space.rye_cmd()
        .arg("self")
        .arg("install")
        .arg("--yes"),
        @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Welcome to Rye!

    This installer will install rye to [RYE_HOME]
    This path can be changed by exporting the RYE_HOME environment variable.

    Details:
      Rye Version: 0.28.0
      Platform: linux (x86_64)

    Installed binary to [RYE_HOME]/shims/rye
    Updated self-python installation at [RYE_HOME]/self

    The rye directory [RYE_HOME]/shims was not detected on PATH.
    It is highly recommended that you add it.
    Added to PATH.
    note: for this to take effect you will need to restart your shell or run this manually:

        source "[RYE_HOME]/env"

    For more information read https://rye-up.com/guide/installation/

    All done!

    ----- stderr -----
    "###
    );

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

    space.write(
        "src/my_project/__init__.py",
        r#"
import sysconfig
def main():
    cc = sysconfig.get_config_var('CC')
    linkcc = sysconfig.get_config_var('LINKCC')
    if 'musl' in cc and 'musl' in linkcc:
        return 0
    else:
        return 1
"#,
    );

    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("my-project"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    "###);
}

#[test]
#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn test_bootstrap_linux_gnu() {
    let space = Space::new();
    space.init_script("my-project");

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

    rye_cmd_snapshot!(space.rye_cmd().arg("run"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    my-project
    python
    python3
    python3.12

    "###);

    // NOTE: Due to #726 hello will currently exit with 1 and print to stderr.
    rye_cmd_snapshot!(space.rye_cmd().arg("run").arg("my-project"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Hello from my-project!

    ----- stderr -----
    "###);
}
