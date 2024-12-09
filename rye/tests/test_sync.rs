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
    Python version: cpython@3.12.8
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    Resolved 1 package in [EXECUTION_TIME]
    Prepared 1 package in [EXECUTION_TIME]
    Installed 1 package in [EXECUTION_TIME]
     + my-project==0.1.0 (from file:[TEMP_PATH]/project)
    "###);

    // is the prompt set?
    #[cfg(unix)]
    {
        let script = space.venv_path().join("bin/activate");
        let contents = fs::read_to_string(script).unwrap();
        assert!(contents.contains("VIRTUAL_ENV_PROMPT=\"(my-project) \""));
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
    Python version: cpython@3.12.8
    Added flask==3.0.0 as regular dependency
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
    Resolved 9 packages in [EXECUTION_TIME]
    Prepared 9 packages in [EXECUTION_TIME]
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
    Python version: cpython@3.12.8
    Added flask==3.0.0 as regular dependency
    Added colorama>=0.4.6 as regular dependency
    Reusing already existing virtualenv
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    Resolved 9 packages in [EXECUTION_TIME]
    Prepared 9 packages in [EXECUTION_TIME]
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

// TODO(charlie): This started failing on Windows in https://github.com/astral-sh/rye/pull/1347,
// likely due to a difference in path canonicalization.
#[test]
#[cfg(unix)]
fn test_autosync_remember() {
    // remove the dependency source markers since they are instable between platforms
    let mut settings = Settings::clone_current();
    settings.add_filter(r"(?m)^\s+# via .*\r?\n", "");
    settings.add_filter(r"(?m)^(\s+)\d+\.\d+s(   \d+ms)?", "$1[TIMING]");
    let _guard = settings.bind_to_scope();

    let space = Space::new();
    space.init("my-project");
    rye_cmd_snapshot!(space.rye_cmd().arg("sync").arg("--with-sources").arg("--all-features"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Initializing new virtualenv in [TEMP_PATH]/project/.venv
    Python version: cpython@3.12.8
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    Resolved 1 package in [EXECUTION_TIME]
    Prepared 1 package in [EXECUTION_TIME]
    Installed 1 package in [EXECUTION_TIME]
     + my-project==0.1.0 (from file:[TEMP_PATH]/project)
    "###);

    rye_cmd_snapshot!(space.rye_cmd()
        .arg("add").arg("--optional=web").arg("flask==3.0.0").arg("colorama"),
        @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Added flask==3.0.0 as optional (web) dependency
    Added colorama>=0.4.6 as optional (web) dependency
    Reusing already existing virtualenv
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    Resolved 9 packages in [EXECUTION_TIME]
    Prepared 9 packages in [EXECUTION_TIME]
    Uninstalled 1 package in [EXECUTION_TIME]
    Installed 9 packages in [EXECUTION_TIME]
     + blinker==1.7.0
     + click==8.1.7
     + colorama==0.4.6
     + flask==3.0.0
     + itsdangerous==2.1.2
     + jinja2==3.1.2
     + markupsafe==2.1.3
     ~ my-project==0.1.0 (from file:[TEMP_PATH]/project)
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
    #   generate-hashes: false
    #   universal: false

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

    rye_cmd_snapshot!(space.rye_cmd().arg("add").arg("urllib3"),
        @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Added urllib3>=2.1.0 as regular dependency
    Reusing already existing virtualenv
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    Resolved 10 packages in [EXECUTION_TIME]
    Prepared 2 packages in [EXECUTION_TIME]
    Uninstalled 1 package in [EXECUTION_TIME]
    Installed 2 packages in [EXECUTION_TIME]
     ~ my-project==0.1.0 (from file:[TEMP_PATH]/project)
     + urllib3==2.1.0
    "###);

    // would be nice to assert on the non quiet output here but unfortunately
    // on CI we seem to have some flakage on this command with regards to
    // rebuilding the editable.
    rye_cmd_snapshot!(space.rye_cmd().arg("sync").arg("-q"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
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
    #   generate-hashes: false
    #   universal: false

    --index-url https://pypi.org/simple/

    -e file:.
    blinker==1.7.0
    click==8.1.7
    colorama==0.4.6
    flask==3.0.0
    itsdangerous==2.1.2
    jinja2==3.1.2
    markupsafe==2.1.3
    urllib3==2.1.0
    werkzeug==3.0.1
    "###);
}

#[test]
fn test_exclude_hashes() {
    let space = Space::new();
    space.init("my-project");

    fs::write(
        space.project_path().join("pyproject.toml"),
        r###"
        [project]
        name = "exclude-rye-test"
        version = "0.1.0"
        dependencies = ["anyio==4.0.0"]
        readme = "README.md"
        requires-python = ">= 3.8"

        [build-system]
        requires = ["hatchling"]
        build-backend = "hatchling.build"

        [tool.rye]
        generate-hashes = true
        excluded-dependencies = ["idna"]

        [tool.hatch.metadata]
        allow-direct-references = true

        [tool.hatch.build.targets.wheel]
        packages = ["src/exclude_rye_test"]
    "###,
    )
    .unwrap();

    rye_cmd_snapshot!(space.rye_cmd().arg("sync"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Initializing new virtualenv in [TEMP_PATH]/project/.venv
    Python version: cpython@3.12.8
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    Resolved 3 packages in [EXECUTION_TIME]
    Prepared 3 packages in [EXECUTION_TIME]
    Installed 3 packages in [EXECUTION_TIME]
     + anyio==4.0.0
     + exclude-rye-test==0.1.0 (from file:[TEMP_PATH]/project)
     + sniffio==1.3.0
    "###);

    assert_snapshot!(space.read_string(space.project_path().join("requirements.lock")), @r###"
    # generated by rye
    # use `rye lock` or `rye sync` to update this lockfile
    #
    # last locked with the following flags:
    #   pre: false
    #   features: []
    #   all-features: false
    #   with-sources: false
    #   generate-hashes: true
    #   universal: false

    -e file:.
    anyio==4.0.0 \
        --hash=sha256:cfdb2b588b9fc25ede96d8db56ed50848b0b649dca3dd1df0b11f683bb9e0b5f \
        --hash=sha256:f7ed51751b2c2add651e5747c891b47e26d2a21be5d32d9311dfe9692f3e5d7a
        # via exclude-rye-test
    # idna==3.4 (excluded)
        # via anyio
    sniffio==1.3.0 \
        --hash=sha256:e60305c5e5d314f5389259b7f22aaa33d8f7dee49763119234af3755c55b9101 \
        --hash=sha256:eecefdce1e5bbfb7ad2eeaabf7c1eeb404d7757c379bd1f7e5cce9d8bf425384
        # via anyio
    "###);
}

#[test]
fn test_lockfile() {
    let space = Space::new();
    space.init("my-project");

    rye_cmd_snapshot!(space.rye_cmd().arg("add").arg("anyio==4.0.0"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Initializing new virtualenv in [TEMP_PATH]/project/.venv
    Python version: cpython@3.12.8
    Added anyio==4.0.0 as regular dependency
    Reusing already existing virtualenv
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    Resolved 4 packages in [EXECUTION_TIME]
    Prepared 4 packages in [EXECUTION_TIME]
    Installed 4 packages in [EXECUTION_TIME]
     + anyio==4.0.0
     + idna==3.4
     + my-project==0.1.0 (from file:[TEMP_PATH]/project)
     + sniffio==1.3.0
    "###);

    assert_snapshot!(space.read_string(space.project_path().join("requirements.lock")), @r###"
    # generated by rye
    # use `rye lock` or `rye sync` to update this lockfile
    #
    # last locked with the following flags:
    #   pre: false
    #   features: []
    #   all-features: false
    #   with-sources: false
    #   generate-hashes: false
    #   universal: false

    -e file:.
    anyio==4.0.0
        # via my-project
    idna==3.4
        # via anyio
    sniffio==1.3.0
        # via anyio
    "###);
}

#[test]
fn test_generate_hashes() {
    let space = Space::new();
    space.init("my-project");

    fs::write(
        space.project_path().join("pyproject.toml"),
        r###"
        [project]
        name = "exclude-rye-test"
        version = "0.1.0"
        dependencies = ["anyio==4.0.0"]
        readme = "README.md"
        requires-python = ">= 3.8"

        [build-system]
        requires = ["hatchling"]
        build-backend = "hatchling.build"

        [tool.rye]
        generate-hashes = true

        [tool.hatch.metadata]
        allow-direct-references = true

        [tool.hatch.build.targets.wheel]
        packages = ["src/exclude_rye_test"]
    "###,
    )
    .unwrap();

    rye_cmd_snapshot!(space.rye_cmd().arg("sync"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Initializing new virtualenv in [TEMP_PATH]/project/.venv
    Python version: cpython@3.12.8
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    Resolved 4 packages in [EXECUTION_TIME]
    Prepared 4 packages in [EXECUTION_TIME]
    Installed 4 packages in [EXECUTION_TIME]
     + anyio==4.0.0
     + exclude-rye-test==0.1.0 (from file:[TEMP_PATH]/project)
     + idna==3.4
     + sniffio==1.3.0
    "###);

    assert_snapshot!(space.read_string(space.project_path().join("requirements.lock")), @r###"
    # generated by rye
    # use `rye lock` or `rye sync` to update this lockfile
    #
    # last locked with the following flags:
    #   pre: false
    #   features: []
    #   all-features: false
    #   with-sources: false
    #   generate-hashes: true
    #   universal: false

    -e file:.
    anyio==4.0.0 \
        --hash=sha256:cfdb2b588b9fc25ede96d8db56ed50848b0b649dca3dd1df0b11f683bb9e0b5f \
        --hash=sha256:f7ed51751b2c2add651e5747c891b47e26d2a21be5d32d9311dfe9692f3e5d7a
        # via exclude-rye-test
    idna==3.4 \
        --hash=sha256:814f528e8dead7d329833b91c5faa87d60bf71824cd12a7530b5526063d02cb4 \
        --hash=sha256:90b77e79eaa3eba6de819a0c442c0b4ceefc341a7a2ab77d7562bf49f425c5c2
        # via anyio
    sniffio==1.3.0 \
        --hash=sha256:e60305c5e5d314f5389259b7f22aaa33d8f7dee49763119234af3755c55b9101 \
        --hash=sha256:eecefdce1e5bbfb7ad2eeaabf7c1eeb404d7757c379bd1f7e5cce9d8bf425384
        # via anyio
    "###);
}
