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

    // is the prompt set?
    #[cfg(unix)]
    {
        let script = space.venv_path().join("bin/activate");
        let contents = fs::read_to_string(script).unwrap();
        assert!(contents.contains("VIRTUAL_ENV_PROMPT=\"my-project\""));
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
    Python version: cpython@3.12.2
    Added flask>=3.0.0 as regular dependency
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
fn test_add_autosync() {
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

    rye_cmd_snapshot!(space.rye_cmd()
        .arg("add").arg("--optional=web").arg("flask==3.0.0").arg("colorama"),
        @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Added flask>=3.0.0 as optional (web) dependency
    Added colorama>=0.4.6 as optional (web) dependency
    Reusing already existing virtualenv
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    Installing dependencies
    Done!

    ----- stderr -----
    Built 1 editable in [EXECUTION_TIME]
    Resolved 8 packages in [EXECUTION_TIME]
    Downloaded 8 packages in [EXECUTION_TIME]
    Uninstalled 1 package in [EXECUTION_TIME]
    Installed 9 packages in [EXECUTION_TIME]
     + blinker==1.7.0
     + click==8.1.7
     + colorama==0.4.6
     + flask==3.0.0
     + itsdangerous==2.1.2
     + jinja2==3.1.2
     + markupsafe==2.1.3
     - my-project==0.1.0 (from file:[TEMP_PATH]/project)
     + my-project==0.1.0 (from file:[TEMP_PATH]/project)
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
    rye_cmd_snapshot!(space.rye_cmd().arg("sync").arg("--verbose"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    Python version already downloaded. Skipping.
    Reusing already existing virtualenv
    Generating production lockfile: [TEMP_PATH]/project/requirements.lock
    -e file:.
    blinker==1.7.0
    click==8.1.7
    colorama==0.4.6
        # via
        #   click
        #   my-project
    flask==3.0.0
    itsdangerous==2.1.2
    jinja2==3.1.2
    markupsafe==2.1.3
        # via
        #   jinja2
        #   werkzeug
    werkzeug==3.0.1
    Generating dev lockfile: [TEMP_PATH]/project/requirements-dev.lock
    -e file:.
    blinker==1.7.0
    click==8.1.7
    colorama==0.4.6
        # via
        #   click
        #   my-project
    flask==3.0.0
    itsdangerous==2.1.2
    jinja2==3.1.2
    markupsafe==2.1.3
        # via
        #   jinja2
        #   werkzeug
    werkzeug==3.0.1
    Installing dependencies
    Done!

    ----- stderr -----
     uv::requirements::from_source source=[TEMP_FILE]
     uv::requirements::from_source source=[TEMP_FILE]
     uv_interpreter::interpreter::find_best python_version=Some(PythonVersion(StringVersion { string: "3.12.2", version: "3.12.2" }))
          [TIMING] DEBUG uv_interpreter::interpreter Starting interpreter discovery for Python 3.12.2
          [TIMING] DEBUG uv_interpreter::python_environment Found a virtualenv named .venv at: [TEMP_PATH]/project/.venv
          [TIMING] DEBUG uv_interpreter::interpreter Cached interpreter info for Python 3.12.2, skipping probing: [TEMP_PATH]/project/.venv/Scripts/python.exe
        [TIMING] DEBUG uv::commands::pip_compile Using Python 3.12.2 interpreter at [36mC:[TEMP_PATH]/project/.venv/Scripts/python.exe[39m for builds
        [TIMING] DEBUG uv_client::registry_client Using registry request timeout of 300s
     uv_client::flat_index::from_entries 
     uv_installer::downloader::build_editables 
          [TIMING] DEBUG uv_distribution::source Building (editable) file:[TEMP_PATH]/project
       uv_dispatch::setup_build package_id="file:[TEMP_PATH]/project", subdirectory=None
         uv_resolver::resolver::solve 
              [TIMING] DEBUG uv_resolver::resolver Solving with target Python version 3.12.2
           uv_resolver::resolver::choose_version package=root
           uv_resolver::resolver::get_dependencies package=root, version=0a0.dev0
                [TIMING] DEBUG uv_resolver::resolver Adding direct dependency: hatchling*
           uv_resolver::resolver::choose_version package=hatchling
             uv_resolver::resolver::package_wait package_name=hatchling
         uv_resolver::resolver::process_request request=Versions hatchling
           uv_client::registry_client::simple_api package=hatchling
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/hatchling.rkyv
         uv_resolver::resolver::process_request request=Prefetch hatchling *
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/hatchling.rkyv"
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/hatchling/
           uv_resolver::version_map::from_metadata 
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=hatchling==1.18.0
             uv_client::registry_client::wheel_metadata built_dist=hatchling==1.18.0
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/hatchling/hatchling-1.18.0-py3-none-any.msgpack
                [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of hatchling (*)
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/hatchling/hatchling-1.18.0-py3-none-any.msgpack"
                [TIMING] DEBUG uv_resolver::resolver Selecting: hatchling==1.18.0 (hatchling-1.18.0-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=hatchling, version=1.18.0
             uv_resolver::resolver::distributions_wait package_id=hatchling-1.18.0
                      [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/76/56/8ccca673e2c896931722f876bf040c0b6a7d8c1a128be60516a8a55bb27a/hatchling-1.18.0-py3-none-any.whl.metadata
                [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: editables>=0.3
                [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: packaging>=21.3
                [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: pathspec>=0.10.1
                [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: pluggy>=1.0.0
                [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: trove-classifiers*
           uv_resolver::resolver::choose_version package=editables
             uv_resolver::resolver::package_wait package_name=editables
         uv_resolver::resolver::process_request request=Versions editables
           uv_client::registry_client::simple_api package=editables
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/editables.rkyv
         uv_resolver::resolver::process_request request=Versions packaging
           uv_client::registry_client::simple_api package=packaging
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/editables.rkyv"
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/packaging.rkyv
         uv_resolver::resolver::process_request request=Versions pathspec
           uv_client::registry_client::simple_api package=pathspec
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/pathspec.rkyv
         uv_resolver::resolver::process_request request=Versions pluggy
           uv_client::registry_client::simple_api package=pluggy
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/packaging.rkyv"
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/pathspec.rkyv"
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/pluggy.rkyv
         uv_resolver::resolver::process_request request=Versions trove-classifiers
           uv_client::registry_client::simple_api package=trove-classifiers
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/pluggy.rkyv"
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/trove-classifiers.rkyv
         uv_resolver::resolver::process_request request=Prefetch trove-classifiers *
         uv_resolver::resolver::process_request request=Prefetch pluggy >=1.0.0
         uv_resolver::resolver::process_request request=Prefetch pathspec >=0.10.1
         uv_resolver::resolver::process_request request=Prefetch packaging >=21.3
         uv_resolver::resolver::process_request request=Prefetch editables >=0.3
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/editables/
           uv_resolver::version_map::from_metadata 
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=editables==0.5
             uv_client::registry_client::wheel_metadata built_dist=editables==0.5
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/editables/editables-0.5-py3-none-any.msgpack
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/pathspec/
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/editables/editables-0.5-py3-none-any.msgpack"
           uv_resolver::version_map::from_metadata 
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/trove-classifiers.rkyv"
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=pathspec==0.11.2
             uv_client::registry_client::wheel_metadata built_dist=pathspec==0.11.2
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/pathspec/pathspec-0.11.2-py3-none-any.msgpack
                [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of editables (>=0.3)
                [TIMING] DEBUG uv_resolver::resolver Selecting: editables==0.5 (editables-0.5-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=editables, version=0.5
             uv_resolver::resolver::distributions_wait package_id=editables-0.5
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/packaging/
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/pathspec/pathspec-0.11.2-py3-none-any.msgpack"
           uv_resolver::version_map::from_metadata 
                      [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/6b/be/0f2f4a5e8adc114a02b63d92bf8edbfa24db6fc602fca83c885af2479e0e/editables-0.5-py3-none-any.whl.metadata
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/pluggy/
           uv_resolver::version_map::from_metadata 
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=packaging==23.2
             uv_client::registry_client::wheel_metadata built_dist=packaging==23.2
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/packaging/packaging-23.2-py3-none-any.msgpack
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=pluggy==1.3.0
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/packaging/packaging-23.2-py3-none-any.msgpack"
             uv_client::registry_client::wheel_metadata built_dist=pluggy==1.3.0
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/pluggy/pluggy-1.3.0-py3-none-any.msgpack
                      [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/b4/2a/9b1be29146139ef459188f5e420a66e835dda921208db600b7037093891f/pathspec-0.11.2-py3-none-any.whl.metadata
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/pluggy/pluggy-1.3.0-py3-none-any.msgpack"
           uv_resolver::resolver::choose_version package=packaging
             uv_resolver::resolver::package_wait package_name=packaging
                [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of packaging (>=21.3)
                [TIMING] DEBUG uv_resolver::resolver Selecting: packaging==23.2 (packaging-23.2-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=packaging, version=23.2
             uv_resolver::resolver::distributions_wait package_id=packaging-23.2
                      [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/ec/1a/610693ac4ee14fcdf2d9bf3c493370e4f2ef7ae2e19217d7a237ff42367d/packaging-23.2-py3-none-any.whl.metadata
           uv_resolver::resolver::choose_version package=pathspec
             uv_resolver::resolver::package_wait package_name=pathspec
                [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of pathspec (>=0.10.1)
                [TIMING] DEBUG uv_resolver::resolver Selecting: pathspec==0.11.2 (pathspec-0.11.2-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=pathspec, version=0.11.2
             uv_resolver::resolver::distributions_wait package_id=pathspec-0.11.2
           uv_resolver::resolver::choose_version package=pluggy
             uv_resolver::resolver::package_wait package_name=pluggy
                [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of pluggy (>=1.0.0)
                [TIMING] DEBUG uv_resolver::resolver Selecting: pluggy==1.3.0 (pluggy-1.3.0-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=pluggy, version=1.3.0
             uv_resolver::resolver::distributions_wait package_id=pluggy-1.3.0
                      [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/05/b8/42ed91898d4784546c5f06c60506400548db3f7a4b3fb441cba4e5c17952/pluggy-1.3.0-py3-none-any.whl.metadata
           uv_resolver::resolver::choose_version package=trove-classifiers
             uv_resolver::resolver::package_wait package_name=trove-classifiers
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/trove-classifiers/
           uv_resolver::version_map::from_metadata 
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=trove-classifiers==2023.11.14
             uv_client::registry_client::wheel_metadata built_dist=trove-classifiers==2023.11.14
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/trove-classifiers/trove_classifiers-2023.11.14-py3-none-any.msgpack
                [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of trove-classifiers (*)
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/trove-classifiers/trove_classifiers-2023.11.14-py3-none-any.msgpack"
                [TIMING] DEBUG uv_resolver::resolver Selecting: trove-classifiers==2023.11.14 (trove_classifiers-2023.11.14-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=trove-classifiers, version=2023.11.14
             uv_resolver::resolver::distributions_wait package_id=trove-classifiers-2023.11.14
                      [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/a9/58/3feea94f12f25714f54a1cc14f3760977631d62c70952de3ab4bd0c6bc41/trove_classifiers-2023.11.14-py3-none-any.whl.metadata
         uv_dispatch::install resolution="editables==0.5, trove-classifiers==2023.11.14, pathspec==0.11.2, pluggy==1.3.0, packaging==23.2, hatchling==1.18.0", venv="[TEMP_PATH]//uv-cache/.tmpOHMYZI/.venv"
              [TIMING] DEBUG uv_dispatch Installing in editables==0.5, trove-classifiers==2023.11.14, pathspec==0.11.2, pluggy==1.3.0, packaging==23.2, hatchling==1.18.0 in [TEMP_PATH]/uv-cache/.tmpOHMYZI/.venv
              [TIMING] DEBUG uv_installer::plan Requirement already cached: editables==0.5
              [TIMING] DEBUG uv_installer::plan Requirement already cached: hatchling==1.18.0
              [TIMING] DEBUG uv_installer::plan Requirement already cached: packaging==23.2
              [TIMING] DEBUG uv_installer::plan Requirement already cached: pathspec==0.11.2
              [TIMING] DEBUG uv_installer::plan Requirement already cached: pluggy==1.3.0
              [TIMING] DEBUG uv_installer::plan Requirement already cached: trove-classifiers==2023.11.14
              [TIMING] DEBUG uv_dispatch Installing build requirements: editables==0.5, hatchling==1.18.0, packaging==23.2, pathspec==0.11.2, pluggy==1.3.0, trove-classifiers==2023.11.14
           uv_installer::installer::install num_wheels=6
            [TIMING]  72ms DEBUG uv_build Calling `hatchling.build.get_requires_for_build_editable()`
         uv_build::run_python_script script="get_requires_for_build_editable", python_version=3.12.2
       uv_build::build package_id="file:[TEMP_PATH]/project"
            [TIMING] DEBUG uv_build Calling `hatchling.build.build_editable(metadata_directory=None)`
         uv_build::run_python_script script="build_editable", python_version=3.12.2
          [TIMING] 807ms DEBUG uv_distribution::source Finished building (editable): my-project @ file:[TEMP_PATH]/project
     uv_distribution::unzip::unzip filename="my_project-0.1.0-py3-none-any.whl"
    Built 1 editable in [EXECUTION_TIME]
     uv_resolver::resolver::solve 
          [TIMING] DEBUG uv_resolver::resolver Solving with target Python version 3.12.2
       uv_resolver::resolver::choose_version package=root
       uv_resolver::resolver::get_dependencies package=root, version=0a0.dev0
       uv_resolver::resolver::choose_version package=my-project[web]
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of my-project[web] @ file:[TEMP_PATH]/project (==0.1.0)
       uv_resolver::resolver::get_dependencies package=my-project[web], version=0.1.0
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: colorama>=0.4.6
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: flask>=3.0.0
       uv_resolver::resolver::choose_version package=my-project
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of my-project @ file:[TEMP_PATH]/project (==0.1.0)
       uv_resolver::resolver::get_dependencies package=my-project, version=0.1.0
       uv_resolver::resolver::choose_version package=colorama
         uv_resolver::resolver::package_wait package_name=colorama
     uv_resolver::resolver::process_request request=Versions colorama
       uv_client::registry_client::simple_api package=colorama
         uv_client::cached_client::get_cacheable 
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/colorama.rkyv
     uv_resolver::resolver::process_request request=Versions flask
       uv_client::registry_client::simple_api package=flask
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/colorama.rkyv"
         uv_client::cached_client::get_cacheable 
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/flask.rkyv
     uv_resolver::resolver::process_request request=Prefetch flask >=3.0.0
     uv_resolver::resolver::process_request request=Prefetch colorama >=0.4.6
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/flask.rkyv"
              [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/colorama/
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=colorama==0.4.6
         uv_client::registry_client::wheel_metadata built_dist=colorama==0.4.6
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/colorama/colorama-0.4.6-py2.py3-none-any.msgpack
              [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/flask/
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/colorama/colorama-0.4.6-py2.py3-none-any.msgpack"
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=flask==3.0.0
         uv_client::registry_client::wheel_metadata built_dist=flask==3.0.0
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/flask/flask-3.0.0-py3-none-any.msgpack
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of colorama (>=0.4.6)
            [TIMING] DEBUG uv_resolver::resolver Selecting: colorama==0.4.6 (colorama-0.4.6-py2.py3-none-any.whl)
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/flask/flask-3.0.0-py3-none-any.msgpack"
       uv_resolver::resolver::get_dependencies package=colorama, version=0.4.6
         uv_resolver::resolver::distributions_wait package_id=colorama-0.4.6
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/d1/d6/3965ed04c63042e047cb6a3e6ed1a63a35087b6a609aa3a15ed8ac56c221/colorama-0.4.6-py2.py3-none-any.whl.metadata
       uv_resolver::resolver::choose_version package=flask
         uv_resolver::resolver::package_wait package_name=flask
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of flask (>=3.0.0)
            [TIMING] DEBUG uv_resolver::resolver Selecting: flask==3.0.0 (flask-3.0.0-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=flask, version=3.0.0
         uv_resolver::resolver::distributions_wait package_id=flask-3.0.0
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/36/42/015c23096649b908c809c69388a805a571a3bea44362fe87e33fc3afa01f/flask-3.0.0-py3-none-any.whl.metadata
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: werkzeug>=3.0.0
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: jinja2>=3.1.2
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: itsdangerous>=2.1.2
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: click>=8.1.3
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: blinker>=1.6.2
       uv_resolver::resolver::choose_version package=werkzeug
         uv_resolver::resolver::package_wait package_name=werkzeug
     uv_resolver::resolver::process_request request=Versions werkzeug
       uv_client::registry_client::simple_api package=werkzeug
         uv_client::cached_client::get_cacheable 
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/werkzeug.rkyv
     uv_resolver::resolver::process_request request=Versions jinja2
       uv_client::registry_client::simple_api package=jinja2
         uv_client::cached_client::get_cacheable 
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/werkzeug.rkyv"
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/jinja2.rkyv
     uv_resolver::resolver::process_request request=Versions itsdangerous
       uv_client::registry_client::simple_api package=itsdangerous
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/jinja2.rkyv"
         uv_client::cached_client::get_cacheable 
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/itsdangerous.rkyv
     uv_resolver::resolver::process_request request=Versions click
       uv_client::registry_client::simple_api package=click
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/itsdangerous.rkyv"
         uv_client::cached_client::get_cacheable 
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/click.rkyv
     uv_resolver::resolver::process_request request=Versions blinker
       uv_client::registry_client::simple_api package=blinker
         uv_client::cached_client::get_cacheable 
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/blinker.rkyv
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/click.rkyv"
     uv_resolver::resolver::process_request request=Prefetch blinker >=1.6.2
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/blinker.rkyv"
     uv_resolver::resolver::process_request request=Prefetch click >=8.1.3
     uv_resolver::resolver::process_request request=Prefetch itsdangerous >=2.1.2
     uv_resolver::resolver::process_request request=Prefetch jinja2 >=3.1.2
     uv_resolver::resolver::process_request request=Prefetch werkzeug >=3.0.0
              [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/itsdangerous/
       uv_resolver::version_map::from_metadata 
              [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/jinja2/
       uv_resolver::version_map::from_metadata 
              [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/werkzeug/
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=itsdangerous==2.1.2
         uv_client::registry_client::wheel_metadata built_dist=itsdangerous==2.1.2
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/itsdangerous/itsdangerous-2.1.2-py3-none-any.msgpack
              [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/blinker/
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/itsdangerous/itsdangerous-2.1.2-py3-none-any.msgpack"
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=jinja2==3.1.2
         uv_client::registry_client::wheel_metadata built_dist=jinja2==3.1.2
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/jinja2/jinja2-3.1.2-py3-none-any.msgpack
              [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/click/
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/jinja2/jinja2-3.1.2-py3-none-any.msgpack"
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=werkzeug==3.0.1
         uv_client::registry_client::wheel_metadata built_dist=werkzeug==3.0.1
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/werkzeug/werkzeug-3.0.1-py3-none-any.msgpack
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=blinker==1.7.0
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/werkzeug/werkzeug-3.0.1-py3-none-any.msgpack"
         uv_client::registry_client::wheel_metadata built_dist=blinker==1.7.0
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/blinker/blinker-1.7.0-py3-none-any.msgpack
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=click==8.1.7
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/blinker/blinker-1.7.0-py3-none-any.msgpack"
         uv_client::registry_client::wheel_metadata built_dist=click==8.1.7
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/click/click-8.1.7-py3-none-any.msgpack
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/68/5f/447e04e828f47465eeab35b5d408b7ebaaaee207f48b7136c5a7267a30ae/itsdangerous-2.1.2-py3-none-any.whl.metadata
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/click/click-8.1.7-py3-none-any.msgpack"
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/bc/c3/f068337a370801f372f2f8f6bad74a5c140f6fda3d9de154052708dd3c65/Jinja2-3.1.2-py3-none-any.whl.metadata
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/c3/fc/254c3e9b5feb89ff5b9076a23218dafbc99c96ac5941e900b71206e6313b/werkzeug-3.0.1-py3-none-any.whl.metadata
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/fa/2a/7f3714cbc6356a0efec525ce7a0613d581072ed6eb53eb7b9754f33db807/blinker-1.7.0-py3-none-any.whl.metadata
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of werkzeug (>=3.0.0)
            [TIMING] DEBUG uv_resolver::resolver Selecting: werkzeug==3.0.1 (werkzeug-3.0.1-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=werkzeug, version=3.0.1
         uv_resolver::resolver::distributions_wait package_id=werkzeug-3.0.1
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: markupsafe>=2.1.1
       uv_resolver::resolver::choose_version package=jinja2
         uv_resolver::resolver::package_wait package_name=jinja2
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of jinja2 (>=3.1.2)
            [TIMING] DEBUG uv_resolver::resolver Selecting: jinja2==3.1.2 (Jinja2-3.1.2-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=jinja2, version=3.1.2
         uv_resolver::resolver::distributions_wait package_id=jinja2-3.1.2
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: markupsafe>=2.0
       uv_resolver::resolver::choose_version package=itsdangerous
         uv_resolver::resolver::package_wait package_name=itsdangerous
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of itsdangerous (>=2.1.2)
            [TIMING] DEBUG uv_resolver::resolver Selecting: itsdangerous==2.1.2 (itsdangerous-2.1.2-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=itsdangerous, version=2.1.2
         uv_resolver::resolver::distributions_wait package_id=itsdangerous-2.1.2
       uv_resolver::resolver::choose_version package=click
         uv_resolver::resolver::package_wait package_name=click
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of click (>=8.1.3)
            [TIMING] DEBUG uv_resolver::resolver Selecting: click==8.1.7 (click-8.1.7-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=click, version=8.1.7
         uv_resolver::resolver::distributions_wait package_id=click-8.1.7
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/00/2e/d53fa4befbf2cfa713304affc7ca780ce4fc1fd8710527771b58311a3229/click-8.1.7-py3-none-any.whl.metadata
     uv_resolver::resolver::process_request request=Versions markupsafe
       uv_client::registry_client::simple_api package=markupsafe
         uv_client::cached_client::get_cacheable 
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/markupsafe.rkyv
     uv_resolver::resolver::process_request request=Prefetch markupsafe >=2.1.1
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/markupsafe.rkyv"
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: colorama*
       uv_resolver::resolver::choose_version package=blinker
         uv_resolver::resolver::package_wait package_name=blinker
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of blinker (>=1.6.2)
            [TIMING] DEBUG uv_resolver::resolver Selecting: blinker==1.7.0 (blinker-1.7.0-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=blinker, version=1.7.0
         uv_resolver::resolver::distributions_wait package_id=blinker-1.7.0
       uv_resolver::resolver::choose_version package=markupsafe
         uv_resolver::resolver::package_wait package_name=markupsafe
              [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/markupsafe/
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=markupsafe==2.1.3
         uv_client::registry_client::wheel_metadata built_dist=markupsafe==2.1.3
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/markupsafe/markupsafe-2.1.3-cp312-cp312-win_amd64.msgpack
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of markupsafe (>=2.1.1)
            [TIMING] DEBUG uv_resolver::resolver Selecting: markupsafe==2.1.3 (MarkupSafe-2.1.3-cp312-cp312-win_amd64.whl)
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/markupsafe/markupsafe-2.1.3-cp312-cp312-win_amd64.msgpack"
       uv_resolver::resolver::get_dependencies package=markupsafe, version=2.1.3
         uv_resolver::resolver::distributions_wait package_id=markupsafe-2.1.3
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/44/44/dbaf65876e258facd65f586dde158387ab89963e7f2235551afc9c2e24c2/MarkupSafe-2.1.3-cp312-cp312-win_amd64.whl.metadata
    Resolved 9 packages in [EXECUTION_TIME]
     uv::requirements::from_source source=[TEMP_FILE]
     uv::requirements::from_source source=[TEMP_FILE]
     uv_interpreter::interpreter::find_best python_version=Some(PythonVersion(StringVersion { string: "3.12.2", version: "3.12.2" }))
          [TIMING] DEBUG uv_interpreter::interpreter Starting interpreter discovery for Python 3.12.2
          [TIMING] DEBUG uv_interpreter::python_environment Found a virtualenv named .venv at: [TEMP_PATH]/project/.venv
          [TIMING] DEBUG uv_interpreter::interpreter Cached interpreter info for Python 3.12.2, skipping probing: [TEMP_PATH]/project/.venv/Scripts/python.exe
        [TIMING] DEBUG uv::commands::pip_compile Using Python 3.12.2 interpreter at [36mC:[TEMP_PATH]/project/.venv/Scripts/python.exe[39m for builds
        [TIMING] DEBUG uv_client::registry_client Using registry request timeout of 300s
     uv_client::flat_index::from_entries 
     uv_installer::downloader::build_editables 
          [TIMING] DEBUG uv_distribution::source Building (editable) file:[TEMP_PATH]/project
       uv_dispatch::setup_build package_id="file:[TEMP_PATH]/project", subdirectory=None
         uv_resolver::resolver::solve 
              [TIMING] DEBUG uv_resolver::resolver Solving with target Python version 3.12.2
           uv_resolver::resolver::choose_version package=root
           uv_resolver::resolver::get_dependencies package=root, version=0a0.dev0
                [TIMING] DEBUG uv_resolver::resolver Adding direct dependency: hatchling*
           uv_resolver::resolver::choose_version package=hatchling
             uv_resolver::resolver::package_wait package_name=hatchling
         uv_resolver::resolver::process_request request=Versions hatchling
           uv_client::registry_client::simple_api package=hatchling
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/hatchling.rkyv
         uv_resolver::resolver::process_request request=Prefetch hatchling *
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/hatchling.rkyv"
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/hatchling/
           uv_resolver::version_map::from_metadata 
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=hatchling==1.18.0
             uv_client::registry_client::wheel_metadata built_dist=hatchling==1.18.0
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/hatchling/hatchling-1.18.0-py3-none-any.msgpack
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/hatchling/hatchling-1.18.0-py3-none-any.msgpack"
                [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of hatchling (*)
                [TIMING] DEBUG uv_resolver::resolver Selecting: hatchling==1.18.0 (hatchling-1.18.0-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=hatchling, version=1.18.0
             uv_resolver::resolver::distributions_wait package_id=hatchling-1.18.0
                      [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/76/56/8ccca673e2c896931722f876bf040c0b6a7d8c1a128be60516a8a55bb27a/hatchling-1.18.0-py3-none-any.whl.metadata
                [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: editables>=0.3
                [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: packaging>=21.3
                [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: pathspec>=0.10.1
                [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: pluggy>=1.0.0
                [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: trove-classifiers*
           uv_resolver::resolver::choose_version package=editables
             uv_resolver::resolver::package_wait package_name=editables
         uv_resolver::resolver::process_request request=Versions editables
           uv_client::registry_client::simple_api package=editables
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/editables.rkyv
         uv_resolver::resolver::process_request request=Versions packaging
           uv_client::registry_client::simple_api package=packaging
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/editables.rkyv"
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/packaging.rkyv
         uv_resolver::resolver::process_request request=Versions pathspec
           uv_client::registry_client::simple_api package=pathspec
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/pathspec.rkyv
         uv_resolver::resolver::process_request request=Versions pluggy
           uv_client::registry_client::simple_api package=pluggy
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/packaging.rkyv"
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/pluggy.rkyv
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/pathspec.rkyv"
         uv_resolver::resolver::process_request request=Versions trove-classifiers
           uv_client::registry_client::simple_api package=trove-classifiers
             uv_client::cached_client::get_cacheable 
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/pluggy.rkyv"
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/trove-classifiers.rkyv
         uv_resolver::resolver::process_request request=Prefetch trove-classifiers *
         uv_resolver::resolver::process_request request=Prefetch pluggy >=1.0.0
         uv_resolver::resolver::process_request request=Prefetch pathspec >=0.10.1
         uv_resolver::resolver::process_request request=Prefetch packaging >=21.3
         uv_resolver::resolver::process_request request=Prefetch editables >=0.3
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/editables/
           uv_resolver::version_map::from_metadata 
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/pathspec/
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/trove-classifiers.rkyv"
           uv_resolver::version_map::from_metadata 
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=editables==0.5
             uv_client::registry_client::wheel_metadata built_dist=editables==0.5
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/editables/editables-0.5-py3-none-any.msgpack
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/packaging/
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/editables/editables-0.5-py3-none-any.msgpack"
           uv_resolver::version_map::from_metadata 
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=pathspec==0.11.2
             uv_client::registry_client::wheel_metadata built_dist=pathspec==0.11.2
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/pathspec/pathspec-0.11.2-py3-none-any.msgpack
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/pluggy/
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/pathspec/pathspec-0.11.2-py3-none-any.msgpack"
           uv_resolver::version_map::from_metadata 
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=packaging==23.2
             uv_client::registry_client::wheel_metadata built_dist=packaging==23.2
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/packaging/packaging-23.2-py3-none-any.msgpack
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=pluggy==1.3.0
             uv_client::registry_client::wheel_metadata built_dist=pluggy==1.3.0
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/packaging/packaging-23.2-py3-none-any.msgpack"
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/pluggy/pluggy-1.3.0-py3-none-any.msgpack
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/trove-classifiers/
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/pluggy/pluggy-1.3.0-py3-none-any.msgpack"
           uv_resolver::version_map::from_metadata 
                      [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/6b/be/0f2f4a5e8adc114a02b63d92bf8edbfa24db6fc602fca83c885af2479e0e/editables-0.5-py3-none-any.whl.metadata
                      [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/b4/2a/9b1be29146139ef459188f5e420a66e835dda921208db600b7037093891f/pathspec-0.11.2-py3-none-any.whl.metadata
                      [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/ec/1a/610693ac4ee14fcdf2d9bf3c493370e4f2ef7ae2e19217d7a237ff42367d/packaging-23.2-py3-none-any.whl.metadata
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=trove-classifiers==2023.11.14
             uv_client::registry_client::wheel_metadata built_dist=trove-classifiers==2023.11.14
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/trove-classifiers/trove_classifiers-2023.11.14-py3-none-any.msgpack
                      [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/05/b8/42ed91898d4784546c5f06c60506400548db3f7a4b3fb441cba4e5c17952/pluggy-1.3.0-py3-none-any.whl.metadata
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/trove-classifiers/trove_classifiers-2023.11.14-py3-none-any.msgpack"
                [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of editables (>=0.3)
                [TIMING] DEBUG uv_resolver::resolver Selecting: editables==0.5 (editables-0.5-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=editables, version=0.5
             uv_resolver::resolver::distributions_wait package_id=editables-0.5
           uv_resolver::resolver::choose_version package=packaging
             uv_resolver::resolver::package_wait package_name=packaging
                [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of packaging (>=21.3)
                [TIMING] DEBUG uv_resolver::resolver Selecting: packaging==23.2 (packaging-23.2-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=packaging, version=23.2
             uv_resolver::resolver::distributions_wait package_id=packaging-23.2
           uv_resolver::resolver::choose_version package=pathspec
             uv_resolver::resolver::package_wait package_name=pathspec
                [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of pathspec (>=0.10.1)
                [TIMING] DEBUG uv_resolver::resolver Selecting: pathspec==0.11.2 (pathspec-0.11.2-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=pathspec, version=0.11.2
             uv_resolver::resolver::distributions_wait package_id=pathspec-0.11.2
           uv_resolver::resolver::choose_version package=pluggy
             uv_resolver::resolver::package_wait package_name=pluggy
                [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of pluggy (>=1.0.0)
                [TIMING] DEBUG uv_resolver::resolver Selecting: pluggy==1.3.0 (pluggy-1.3.0-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=pluggy, version=1.3.0
             uv_resolver::resolver::distributions_wait package_id=pluggy-1.3.0
           uv_resolver::resolver::choose_version package=trove-classifiers
             uv_resolver::resolver::package_wait package_name=trove-classifiers
                [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of trove-classifiers (*)
                [TIMING] DEBUG uv_resolver::resolver Selecting: trove-classifiers==2023.11.14 (trove_classifiers-2023.11.14-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=trove-classifiers, version=2023.11.14
             uv_resolver::resolver::distributions_wait package_id=trove-classifiers-2023.11.14
                      [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/a9/58/3feea94f12f25714f54a1cc14f3760977631d62c70952de3ab4bd0c6bc41/trove_classifiers-2023.11.14-py3-none-any.whl.metadata
         uv_dispatch::install resolution="editables==0.5, trove-classifiers==2023.11.14, pathspec==0.11.2, pluggy==1.3.0, packaging==23.2, hatchling==1.18.0", venv="[TEMP_PATH]//uv-cache/.tmptfQhdB/.venv"
              [TIMING] DEBUG uv_dispatch Installing in editables==0.5, trove-classifiers==2023.11.14, pathspec==0.11.2, pluggy==1.3.0, packaging==23.2, hatchling==1.18.0 in [TEMP_PATH]/uv-cache/.tmptfQhdB/.venv
              [TIMING] DEBUG uv_installer::plan Requirement already cached: editables==0.5
              [TIMING] DEBUG uv_installer::plan Requirement already cached: hatchling==1.18.0
              [TIMING] DEBUG uv_installer::plan Requirement already cached: packaging==23.2
              [TIMING] DEBUG uv_installer::plan Requirement already cached: pathspec==0.11.2
              [TIMING] DEBUG uv_installer::plan Requirement already cached: pluggy==1.3.0
              [TIMING] DEBUG uv_installer::plan Requirement already cached: trove-classifiers==2023.11.14
              [TIMING] DEBUG uv_dispatch Installing build requirements: editables==0.5, hatchling==1.18.0, packaging==23.2, pathspec==0.11.2, pluggy==1.3.0, trove-classifiers==2023.11.14
           uv_installer::installer::install num_wheels=6
            [TIMING]  39ms DEBUG uv_build Calling `hatchling.build.get_requires_for_build_editable()`
         uv_build::run_python_script script="get_requires_for_build_editable", python_version=3.12.2
       uv_build::build package_id="file:[TEMP_PATH]/project"
            [TIMING] DEBUG uv_build Calling `hatchling.build.build_editable(metadata_directory=None)`
         uv_build::run_python_script script="build_editable", python_version=3.12.2
          [TIMING] 555ms DEBUG uv_distribution::source Finished building (editable): my-project @ file:[TEMP_PATH]/project
     uv_distribution::unzip::unzip filename="my_project-0.1.0-py3-none-any.whl"
    Built 1 editable in [EXECUTION_TIME]
     uv_resolver::resolver::solve 
          [TIMING] DEBUG uv_resolver::resolver Solving with target Python version 3.12.2
       uv_resolver::resolver::choose_version package=root
       uv_resolver::resolver::get_dependencies package=root, version=0a0.dev0
       uv_resolver::resolver::choose_version package=my-project[web]
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of my-project[web] @ file:[TEMP_PATH]/project (==0.1.0)
       uv_resolver::resolver::get_dependencies package=my-project[web], version=0.1.0
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: colorama>=0.4.6
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: flask>=3.0.0
       uv_resolver::resolver::choose_version package=my-project
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of my-project @ file:[TEMP_PATH]/project (==0.1.0)
       uv_resolver::resolver::get_dependencies package=my-project, version=0.1.0
       uv_resolver::resolver::choose_version package=colorama
         uv_resolver::resolver::package_wait package_name=colorama
     uv_resolver::resolver::process_request request=Versions colorama
       uv_client::registry_client::simple_api package=colorama
         uv_client::cached_client::get_cacheable 
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/colorama.rkyv
     uv_resolver::resolver::process_request request=Versions flask
       uv_client::registry_client::simple_api package=flask
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/colorama.rkyv"
         uv_client::cached_client::get_cacheable 
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/flask.rkyv
     uv_resolver::resolver::process_request request=Prefetch flask >=3.0.0
     uv_resolver::resolver::process_request request=Prefetch colorama >=0.4.6
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/flask.rkyv"
              [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/colorama/
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=colorama==0.4.6
         uv_client::registry_client::wheel_metadata built_dist=colorama==0.4.6
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/colorama/colorama-0.4.6-py2.py3-none-any.msgpack
              [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/flask/
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/colorama/colorama-0.4.6-py2.py3-none-any.msgpack"
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=flask==3.0.0
         uv_client::registry_client::wheel_metadata built_dist=flask==3.0.0
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/flask/flask-3.0.0-py3-none-any.msgpack
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of colorama (>=0.4.6)
            [TIMING] DEBUG uv_resolver::resolver Selecting: colorama==0.4.6 (colorama-0.4.6-py2.py3-none-any.whl)
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/flask/flask-3.0.0-py3-none-any.msgpack"
       uv_resolver::resolver::get_dependencies package=colorama, version=0.4.6
         uv_resolver::resolver::distributions_wait package_id=colorama-0.4.6
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/d1/d6/3965ed04c63042e047cb6a3e6ed1a63a35087b6a609aa3a15ed8ac56c221/colorama-0.4.6-py2.py3-none-any.whl.metadata
       uv_resolver::resolver::choose_version package=flask
         uv_resolver::resolver::package_wait package_name=flask
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of flask (>=3.0.0)
            [TIMING] DEBUG uv_resolver::resolver Selecting: flask==3.0.0 (flask-3.0.0-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=flask, version=3.0.0
         uv_resolver::resolver::distributions_wait package_id=flask-3.0.0
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/36/42/015c23096649b908c809c69388a805a571a3bea44362fe87e33fc3afa01f/flask-3.0.0-py3-none-any.whl.metadata
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: werkzeug>=3.0.0
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: jinja2>=3.1.2
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: itsdangerous>=2.1.2
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: click>=8.1.3
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: blinker>=1.6.2
       uv_resolver::resolver::choose_version package=werkzeug
         uv_resolver::resolver::package_wait package_name=werkzeug
     uv_resolver::resolver::process_request request=Versions werkzeug
       uv_client::registry_client::simple_api package=werkzeug
         uv_client::cached_client::get_cacheable 
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/werkzeug.rkyv
     uv_resolver::resolver::process_request request=Versions jinja2
       uv_client::registry_client::simple_api package=jinja2
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/werkzeug.rkyv"
         uv_client::cached_client::get_cacheable 
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/jinja2.rkyv
     uv_resolver::resolver::process_request request=Versions itsdangerous
       uv_client::registry_client::simple_api package=itsdangerous
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/jinja2.rkyv"
         uv_client::cached_client::get_cacheable 
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/itsdangerous.rkyv
     uv_resolver::resolver::process_request request=Versions click
       uv_client::registry_client::simple_api package=click
         uv_client::cached_client::get_cacheable 
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/itsdangerous.rkyv"
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/click.rkyv
     uv_resolver::resolver::process_request request=Versions blinker
       uv_client::registry_client::simple_api package=blinker
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/click.rkyv"
         uv_client::cached_client::get_cacheable 
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/blinker.rkyv
     uv_resolver::resolver::process_request request=Prefetch blinker >=1.6.2
     uv_resolver::resolver::process_request request=Prefetch click >=8.1.3
     uv_resolver::resolver::process_request request=Prefetch itsdangerous >=2.1.2
     uv_resolver::resolver::process_request request=Prefetch jinja2 >=3.1.2
     uv_resolver::resolver::process_request request=Prefetch werkzeug >=3.0.0
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/blinker.rkyv"
              [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/jinja2/
       uv_resolver::version_map::from_metadata 
              [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/itsdangerous/
       uv_resolver::version_map::from_metadata 
              [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/werkzeug/
       uv_resolver::version_map::from_metadata 
              [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/blinker/
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=jinja2==3.1.2
         uv_client::registry_client::wheel_metadata built_dist=jinja2==3.1.2
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/jinja2/jinja2-3.1.2-py3-none-any.msgpack
              [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/click/
       uv_resolver::version_map::from_metadata 
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/jinja2/jinja2-3.1.2-py3-none-any.msgpack"
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=itsdangerous==2.1.2
         uv_client::registry_client::wheel_metadata built_dist=itsdangerous==2.1.2
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/itsdangerous/itsdangerous-2.1.2-py3-none-any.msgpack
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/itsdangerous/itsdangerous-2.1.2-py3-none-any.msgpack"
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=werkzeug==3.0.1
         uv_client::registry_client::wheel_metadata built_dist=werkzeug==3.0.1
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/werkzeug/werkzeug-3.0.1-py3-none-any.msgpack
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/werkzeug/werkzeug-3.0.1-py3-none-any.msgpack"
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=blinker==1.7.0
         uv_client::registry_client::wheel_metadata built_dist=blinker==1.7.0
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/blinker/blinker-1.7.0-py3-none-any.msgpack
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/blinker/blinker-1.7.0-py3-none-any.msgpack"
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=click==8.1.7
         uv_client::registry_client::wheel_metadata built_dist=click==8.1.7
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/click/click-8.1.7-py3-none-any.msgpack
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/bc/c3/f068337a370801f372f2f8f6bad74a5c140f6fda3d9de154052708dd3c65/Jinja2-3.1.2-py3-none-any.whl.metadata
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/click/click-8.1.7-py3-none-any.msgpack"
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/68/5f/447e04e828f47465eeab35b5d408b7ebaaaee207f48b7136c5a7267a30ae/itsdangerous-2.1.2-py3-none-any.whl.metadata
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/c3/fc/254c3e9b5feb89ff5b9076a23218dafbc99c96ac5941e900b71206e6313b/werkzeug-3.0.1-py3-none-any.whl.metadata
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/fa/2a/7f3714cbc6356a0efec525ce7a0613d581072ed6eb53eb7b9754f33db807/blinker-1.7.0-py3-none-any.whl.metadata
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of werkzeug (>=3.0.0)
            [TIMING] DEBUG uv_resolver::resolver Selecting: werkzeug==3.0.1 (werkzeug-3.0.1-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=werkzeug, version=3.0.1
         uv_resolver::resolver::distributions_wait package_id=werkzeug-3.0.1
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: markupsafe>=2.1.1
       uv_resolver::resolver::choose_version package=jinja2
         uv_resolver::resolver::package_wait package_name=jinja2
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of jinja2 (>=3.1.2)
            [TIMING] DEBUG uv_resolver::resolver Selecting: jinja2==3.1.2 (Jinja2-3.1.2-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=jinja2, version=3.1.2
         uv_resolver::resolver::distributions_wait package_id=jinja2-3.1.2
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: markupsafe>=2.0
       uv_resolver::resolver::choose_version package=itsdangerous
         uv_resolver::resolver::package_wait package_name=itsdangerous
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of itsdangerous (>=2.1.2)
            [TIMING] DEBUG uv_resolver::resolver Selecting: itsdangerous==2.1.2 (itsdangerous-2.1.2-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=itsdangerous, version=2.1.2
         uv_resolver::resolver::distributions_wait package_id=itsdangerous-2.1.2
       uv_resolver::resolver::choose_version package=click
         uv_resolver::resolver::package_wait package_name=click
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of click (>=8.1.3)
            [TIMING] DEBUG uv_resolver::resolver Selecting: click==8.1.7 (click-8.1.7-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=click, version=8.1.7
         uv_resolver::resolver::distributions_wait package_id=click-8.1.7
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/00/2e/d53fa4befbf2cfa713304affc7ca780ce4fc1fd8710527771b58311a3229/click-8.1.7-py3-none-any.whl.metadata
     uv_resolver::resolver::process_request request=Versions markupsafe
       uv_client::registry_client::simple_api package=markupsafe
         uv_client::cached_client::get_cacheable 
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/markupsafe.rkyv
     uv_resolver::resolver::process_request request=Prefetch markupsafe >=2.1.1
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/markupsafe.rkyv"
            [TIMING] DEBUG uv_resolver::resolver Adding transitive dependency: colorama*
       uv_resolver::resolver::choose_version package=blinker
         uv_resolver::resolver::package_wait package_name=blinker
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of blinker (>=1.6.2)
            [TIMING] DEBUG uv_resolver::resolver Selecting: blinker==1.7.0 (blinker-1.7.0-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=blinker, version=1.7.0
         uv_resolver::resolver::distributions_wait package_id=blinker-1.7.0
       uv_resolver::resolver::choose_version package=markupsafe
         uv_resolver::resolver::package_wait package_name=markupsafe
              [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/markupsafe/
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=markupsafe==2.1.3
         uv_client::registry_client::wheel_metadata built_dist=markupsafe==2.1.3
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/markupsafe/markupsafe-2.1.3-cp312-cp312-win_amd64.msgpack
            [TIMING] DEBUG uv_resolver::resolver Searching for a compatible version of markupsafe (>=2.1.1)
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/markupsafe/markupsafe-2.1.3-cp312-cp312-win_amd64.msgpack"
            [TIMING] DEBUG uv_resolver::resolver Selecting: markupsafe==2.1.3 (MarkupSafe-2.1.3-cp312-cp312-win_amd64.whl)
       uv_resolver::resolver::get_dependencies package=markupsafe, version=2.1.3
         uv_resolver::resolver::distributions_wait package_id=markupsafe-2.1.3
                  [TIMING] DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/44/44/dbaf65876e258facd65f586dde158387ab89963e7f2235551afc9c2e24c2/MarkupSafe-2.1.3-cp312-cp312-win_amd64.whl.metadata
    Resolved 9 packages in [EXECUTION_TIME]
     uv::requirements::from_source source=[TEMP_PATH]/project/requirements-dev.lock
        [TIMING] DEBUG uv_interpreter::python_environment Found a virtualenv through VIRTUAL_ENV at: [TEMP_PATH]/project/.venv
        [TIMING] DEBUG uv_interpreter::interpreter Cached interpreter info for Python 3.12.2, skipping probing: [TEMP_PATH]/project/.venv/Scripts/python.exe
        [TIMING] DEBUG uv::commands::pip_sync Using Python 3.12.2 environment at [36mC:[TEMP_PATH]/project/.venv/Scripts/python.exe[39m
        [TIMING] DEBUG uv_client::registry_client Using registry request timeout of 300s
     uv_client::flat_index::from_entries 
        [TIMING] DEBUG uv_installer::plan Treating editable requirement as immutable: my-project==0.1.0 (from file:[TEMP_PATH]/project)
        [TIMING] DEBUG uv_installer::plan Requirement already satisfied: blinker==1.7.0
        [TIMING] DEBUG uv_installer::plan Requirement already satisfied: click==8.1.7
        [TIMING] DEBUG uv_installer::plan Requirement already satisfied: colorama==0.4.6
        [TIMING] DEBUG uv_installer::plan Requirement already satisfied: flask==3.0.0
        [TIMING] DEBUG uv_installer::plan Requirement already satisfied: itsdangerous==2.1.2
        [TIMING] DEBUG uv_installer::plan Requirement already satisfied: jinja2==3.1.2
        [TIMING] DEBUG uv_installer::plan Requirement already satisfied: markupsafe==2.1.3
        [TIMING] DEBUG uv_installer::plan Requirement already satisfied: werkzeug==3.0.1
    Audited 9 packages in [EXECUTION_TIME]
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
}
