use std::fs;

use toml_edit::{value, ArrayOfTables, Table};

use crate::common::{get_fixtures_dir, rye_cmd_snapshot, Space};

mod common;

#[test]
fn test_add_flask() {
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
fn test_add_flask_dotenv() {
    let space = Space::new();
    space.init("my-project");
    // add colorama to ensure we have this as a dependency on all platforms
    rye_cmd_snapshot!(space.rye_cmd().arg("add").arg("flask[dotenv]").arg("colorama"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Initializing new virtualenv in [TEMP_PATH]/project/.venv
    Python version: cpython@3.12.2
    Added flask[dotenv]>=3.0.0 as regular dependency
    Added colorama>=0.4.6 as regular dependency
    Reusing already existing virtualenv
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    Built 1 editable in [EXECUTION_TIME]
    Resolved 9 packages in [EXECUTION_TIME]
    Downloaded 9 packages in [EXECUTION_TIME]
    Installed 10 packages in [EXECUTION_TIME]
     + blinker==1.7.0
     + click==8.1.7
     + colorama==0.4.6
     + flask==3.0.0
     + itsdangerous==2.1.2
     + jinja2==3.1.2
     + markupsafe==2.1.3
     + my-project==0.1.0 (from file:[TEMP_PATH]/project)
     + python-dotenv==1.0.0
     + werkzeug==3.0.1
    "###);

    space.load_toml("pyproject.toml", |doc| {
        let deps = doc["project"]["dependencies"].as_array().unwrap();
        assert!(deps
            .iter()
            .any(|x| x.as_str() == Some("flask[dotenv]>=3.0.0")));
    });
}

#[test]
fn test_add_from_find_links() {
    let space = Space::new();
    space.init("my-project");
    space.edit_toml("pyproject.toml", |doc| {
        let mut source = Table::new();
        source["name"] = value("extra");
        source["type"] = value("find-links");
        source["url"] = value("https://download.pytorch.org/whl/torch_stable.html");
        let mut sources = ArrayOfTables::new();
        sources.push(source);
        doc["tool"]["rye"]["sources"] = value(sources.into_array());
    });

    rye_cmd_snapshot!(space.rye_cmd().arg("add").arg("tqdm").arg("colorama"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Initializing new virtualenv in [TEMP_PATH]/project/.venv
    Python version: cpython@3.12.2
    Added tqdm>=4.66.1 as regular dependency
    Added colorama>=0.4.6 as regular dependency
    Reusing already existing virtualenv
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    Built 1 editable in [EXECUTION_TIME]
    Resolved 2 packages in [EXECUTION_TIME]
    Downloaded 2 packages in [EXECUTION_TIME]
    Installed 3 packages in [EXECUTION_TIME]
     + colorama==0.4.6
     + my-project==0.1.0 (from file:[TEMP_PATH]/project)
     + tqdm==4.66.1
    "###);
}

#[test]
fn test_add_flask_wrong_venv_exported() {
    let space = Space::new();
    space.init("my-project");
    let fake_venv = space.project_path().join("fake-venv");
    fs::create_dir_all(&fake_venv).unwrap();
    // add colorama to ensure we have this as a dependency on all platforms
    rye_cmd_snapshot!(space.rye_cmd().arg("add").arg("flask==3.0.0").arg("colorama").env("VIRTUAL_ENV", fake_venv.as_os_str()), @r###"
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
    fs::remove_dir_all(&fake_venv).unwrap();
}

#[test]
fn test_add_from_file_path() {
    let space = Space::new();
    space.init("my-project");
    let fake_venv = space.project_path().join("fake-venv");
    fs::create_dir_all(&fake_venv).unwrap();

    let wheel_path = get_fixtures_dir()
        .join("wheels")
        .join("twine-1.5.0-py2.py3-none-any.whl");
    rye_cmd_snapshot!(space.rye_cmd().arg("add").arg("twine").arg("--path").arg(wheel_path).env("VIRTUAL_ENV", fake_venv.as_os_str()), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Initializing new virtualenv in [TEMP_PATH]/project/.venv
    Python version: cpython@3.12.2
    Added twine @ file:[FIXTURES]/wheels/twine-1.5.0-py2.py3-none-any.whl as regular dependency
    Reusing already existing virtualenv
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    Built 1 editable in [EXECUTION_TIME]
    Resolved 7 packages in [EXECUTION_TIME]
    Downloaded 7 packages in [EXECUTION_TIME]
    Installed 8 packages in [EXECUTION_TIME]
     + certifi==2023.11.17
     + charset-normalizer==3.3.2
     + idna==3.4
     + my-project==0.1.0 (from file:[TEMP_PATH]/project)
     + pkginfo==1.9.6
     + requests==2.31.0
     + twine==1.5.0 (from file:[FIXTURES]/wheels/twine-1.5.0-py2.py3-none-any.whl)
     + urllib3==2.1.0
    "###);
    fs::remove_dir_all(&fake_venv).unwrap();
}

#[test]
fn test_add_from_remote_url() {
    let space = Space::new();
    space.init("my-project");
    let fake_venv = space.project_path().join("fake-venv");
    fs::create_dir_all(&fake_venv).unwrap();

    let url = "https://github.com/pypa/pip/archive/1.3.1.zip#sha1=da9234ee9982d4bbb3c72346a6de940a148ea686";
    rye_cmd_snapshot!(space.rye_cmd().arg("add").arg("pip").arg("--url").arg(url).env("VIRTUAL_ENV", fake_venv.as_os_str()), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Initializing new virtualenv in [TEMP_PATH]/project/.venv
    Python version: cpython@3.12.2
    Added pip @ https://github.com/pypa/pip/archive/1.3.1.zip#sha1=da9234ee9982d4bbb3c72346a6de940a148ea686 as regular dependency
    Reusing already existing virtualenv
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    Built 1 editable in [EXECUTION_TIME]
    Resolved 1 package in [EXECUTION_TIME]
    Downloaded 1 package in [EXECUTION_TIME]
    Installed 2 packages in [EXECUTION_TIME]
     + my-project==0.1.0 (from file:[TEMP_PATH]/project)
     + pip==1.3.1 (from https://github.com/pypa/pip/archive/1.3.1.zip#sha1=da9234ee9982d4bbb3c72346a6de940a148ea686)
    "###);
    fs::remove_dir_all(&fake_venv).unwrap();
}
