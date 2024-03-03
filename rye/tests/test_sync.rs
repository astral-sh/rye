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
          0.006278s   0ms DEBUG uv_interpreter::interpreter Starting interpreter discovery for Python 3.12.2
          0.006552s   0ms DEBUG uv_interpreter::python_environment Found a virtualenv named .venv at: [TEMP_PATH]/project/.venv
          0.006851s   0ms DEBUG uv_interpreter::interpreter Cached interpreter info for Python 3.12.2, skipping probing: [TEMP_PATH]/project/.venv/Scripts/python.exe
        0.006871s DEBUG uv::commands::pip_compile Using Python 3.12.2 interpreter at [36mC:[TEMP_PATH]/project/.venv/Scripts/python.exe[39m for builds
        0.006959s DEBUG uv_client::registry_client Using registry request timeout of 300s
     uv_client::flat_index::from_entries 
     uv_installer::downloader::build_editables 
          0.010557s   0ms DEBUG uv_distribution::source Building (editable) file:[TEMP_PATH]/project
       uv_dispatch::setup_build package_id="file:[TEMP_PATH]/project", subdirectory=None
         uv_resolver::resolver::solve 
              0.018587s   0ms DEBUG uv_resolver::resolver Solving with target Python version 3.12.2
           uv_resolver::resolver::choose_version package=root
           uv_resolver::resolver::get_dependencies package=root, version=0a0.dev0
                0.018689s   0ms DEBUG uv_resolver::resolver Adding direct dependency: hatchling*
           uv_resolver::resolver::choose_version package=hatchling
             uv_resolver::resolver::package_wait package_name=hatchling
         uv_resolver::resolver::process_request request=Versions hatchling
           uv_client::registry_client::simple_api package=hatchling
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/hatchling.rkyv
         uv_resolver::resolver::process_request request=Prefetch hatchling *
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/hatchling.rkyv"
                  0.019360s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/hatchling/
           uv_resolver::version_map::from_metadata 
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=hatchling==1.18.0
             uv_client::registry_client::wheel_metadata built_dist=hatchling==1.18.0
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/hatchling/hatchling-1.18.0-py3-none-any.msgpack
                0.019619s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of hatchling (*)
                0.019629s   0ms DEBUG uv_resolver::resolver Selecting: hatchling==1.18.0 (hatchling-1.18.0-py3-none-any.whl)
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/hatchling/hatchling-1.18.0-py3-none-any.msgpack"
           uv_resolver::resolver::get_dependencies package=hatchling, version=1.18.0
             uv_resolver::resolver::distributions_wait package_id=hatchling-1.18.0
                      0.019857s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/76/56/8ccca673e2c896931722f876bf040c0b6a7d8c1a128be60516a8a55bb27a/hatchling-1.18.0-py3-none-any.whl.metadata
                0.019939s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: editables>=0.3
                0.019949s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: packaging>=21.3
                0.019954s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: pathspec>=0.10.1
                0.019958s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: pluggy>=1.0.0
                0.019961s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: trove-classifiers*
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
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/pathspec.rkyv"
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/pluggy.rkyv
         uv_resolver::resolver::process_request request=Versions trove-classifiers
           uv_client::registry_client::simple_api package=trove-classifiers
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/trove-classifiers.rkyv
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/pluggy.rkyv"
         uv_resolver::resolver::process_request request=Prefetch trove-classifiers *
         uv_resolver::resolver::process_request request=Prefetch pluggy >=1.0.0
         uv_resolver::resolver::process_request request=Prefetch pathspec >=0.10.1
         uv_resolver::resolver::process_request request=Prefetch packaging >=21.3
         uv_resolver::resolver::process_request request=Prefetch editables >=0.3
                  0.020517s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/editables/
           uv_resolver::version_map::from_metadata 
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/trove-classifiers.rkyv"
                  0.020583s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/packaging/
           uv_resolver::version_map::from_metadata 
                  0.020702s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/pathspec/
           uv_resolver::version_map::from_metadata 
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=editables==0.5
             uv_client::registry_client::wheel_metadata built_dist=editables==0.5
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/editables/editables-0.5-py3-none-any.msgpack
                  0.020920s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/pluggy/
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/editables/editables-0.5-py3-none-any.msgpack"
           uv_resolver::version_map::from_metadata 
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=packaging==23.2
             uv_client::registry_client::wheel_metadata built_dist=packaging==23.2
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/packaging/packaging-23.2-py3-none-any.msgpack
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/packaging/packaging-23.2-py3-none-any.msgpack"
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=pathspec==0.11.2
             uv_client::registry_client::wheel_metadata built_dist=pathspec==0.11.2
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/pathspec/pathspec-0.11.2-py3-none-any.msgpack
                  0.021149s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/trove-classifiers/
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/pathspec/pathspec-0.11.2-py3-none-any.msgpack"
           uv_resolver::version_map::from_metadata 
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=pluggy==1.3.0
             uv_client::registry_client::wheel_metadata built_dist=pluggy==1.3.0
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/pluggy/pluggy-1.3.0-py3-none-any.msgpack
                      0.021298s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/6b/be/0f2f4a5e8adc114a02b63d92bf8edbfa24db6fc602fca83c885af2479e0e/editables-0.5-py3-none-any.whl.metadata
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/pluggy/pluggy-1.3.0-py3-none-any.msgpack"
                      0.021370s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/ec/1a/610693ac4ee14fcdf2d9bf3c493370e4f2ef7ae2e19217d7a237ff42367d/packaging-23.2-py3-none-any.whl.metadata
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=trove-classifiers==2023.11.14
             uv_client::registry_client::wheel_metadata built_dist=trove-classifiers==2023.11.14
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/trove-classifiers/trove_classifiers-2023.11.14-py3-none-any.msgpack
                      0.021479s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/b4/2a/9b1be29146139ef459188f5e420a66e835dda921208db600b7037093891f/pathspec-0.11.2-py3-none-any.whl.metadata
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/trove-classifiers/trove_classifiers-2023.11.14-py3-none-any.msgpack"
                      0.021502s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/05/b8/42ed91898d4784546c5f06c60506400548db3f7a4b3fb441cba4e5c17952/pluggy-1.3.0-py3-none-any.whl.metadata
                0.021537s   1ms DEBUG uv_resolver::resolver Searching for a compatible version of editables (>=0.3)
                0.021545s   1ms DEBUG uv_resolver::resolver Selecting: editables==0.5 (editables-0.5-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=editables, version=0.5
             uv_resolver::resolver::distributions_wait package_id=editables-0.5
           uv_resolver::resolver::choose_version package=packaging
             uv_resolver::resolver::package_wait package_name=packaging
                0.021586s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of packaging (>=21.3)
                0.021590s   0ms DEBUG uv_resolver::resolver Selecting: packaging==23.2 (packaging-23.2-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=packaging, version=23.2
             uv_resolver::resolver::distributions_wait package_id=packaging-23.2
           uv_resolver::resolver::choose_version package=pathspec
             uv_resolver::resolver::package_wait package_name=pathspec
                0.021619s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of pathspec (>=0.10.1)
                0.021626s   0ms DEBUG uv_resolver::resolver Selecting: pathspec==0.11.2 (pathspec-0.11.2-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=pathspec, version=0.11.2
             uv_resolver::resolver::distributions_wait package_id=pathspec-0.11.2
           uv_resolver::resolver::choose_version package=pluggy
             uv_resolver::resolver::package_wait package_name=pluggy
                0.021653s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of pluggy (>=1.0.0)
                0.021658s   0ms DEBUG uv_resolver::resolver Selecting: pluggy==1.3.0 (pluggy-1.3.0-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=pluggy, version=1.3.0
             uv_resolver::resolver::distributions_wait package_id=pluggy-1.3.0
           uv_resolver::resolver::choose_version package=trove-classifiers
             uv_resolver::resolver::package_wait package_name=trove-classifiers
                0.021689s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of trove-classifiers (*)
                0.021693s   0ms DEBUG uv_resolver::resolver Selecting: trove-classifiers==2023.11.14 (trove_classifiers-2023.11.14-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=trove-classifiers, version=2023.11.14
             uv_resolver::resolver::distributions_wait package_id=trove-classifiers-2023.11.14
                      0.021714s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/a9/58/3feea94f12f25714f54a1cc14f3760977631d62c70952de3ab4bd0c6bc41/trove_classifiers-2023.11.14-py3-none-any.whl.metadata
         uv_dispatch::install resolution="editables==0.5, trove-classifiers==2023.11.14, pathspec==0.11.2, pluggy==1.3.0, packaging==23.2, hatchling==1.18.0", venv="[TEMP_PATH]//uv-cache/.tmpifOIFN/.venv"
              0.021829s   0ms DEBUG uv_dispatch Installing in editables==0.5, trove-classifiers==2023.11.14, pathspec==0.11.2, pluggy==1.3.0, packaging==23.2, hatchling==1.18.0 in [TEMP_PATH]/uv-cache/.tmpifOIFN/.venv
              0.022228s   0ms DEBUG uv_installer::plan Requirement already cached: editables==0.5
              0.022609s   0ms DEBUG uv_installer::plan Requirement already cached: hatchling==1.18.0
              0.022802s   0ms DEBUG uv_installer::plan Requirement already cached: packaging==23.2
              0.023076s   1ms DEBUG uv_installer::plan Requirement already cached: pathspec==0.11.2
              0.023346s   1ms DEBUG uv_installer::plan Requirement already cached: pluggy==1.3.0
              0.023665s   1ms DEBUG uv_installer::plan Requirement already cached: trove-classifiers==2023.11.14
              0.023696s   1ms DEBUG uv_dispatch Installing build requirements: editables==0.5, hatchling==1.18.0, packaging==23.2, pathspec==0.11.2, pluggy==1.3.0, trove-classifiers==2023.11.14
           uv_installer::installer::install num_wheels=6
            0.051215s  40ms DEBUG uv_build Calling `hatchling.build.get_requires_for_build_editable()`
         uv_build::run_python_script script="get_requires_for_build_editable", python_version=3.12.2
       uv_build::build package_id="file:[TEMP_PATH]/project"
            0.285867s   0ms DEBUG uv_build Calling `hatchling.build.build_editable(metadata_directory=None)`
         uv_build::run_python_script script="build_editable", python_version=3.12.2
          0.512648s 502ms DEBUG uv_distribution::source Finished building (editable): my-project @ file:[TEMP_PATH]/project
     uv_distribution::unzip::unzip filename="my_project-0.1.0-py3-none-any.whl"
    Built 1 editable in [EXECUTION_TIME]
     uv_resolver::resolver::solve 
          0.516158s   0ms DEBUG uv_resolver::resolver Solving with target Python version 3.12.2
       uv_resolver::resolver::choose_version package=root
       uv_resolver::resolver::get_dependencies package=root, version=0a0.dev0
       uv_resolver::resolver::choose_version package=my-project[web]
            0.516248s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of my-project[web] @ file:[TEMP_PATH]/project (==0.1.0)
       uv_resolver::resolver::get_dependencies package=my-project[web], version=0.1.0
            0.516270s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: colorama>=0.4.6
            0.516279s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: flask>=3.0.0
       uv_resolver::resolver::choose_version package=my-project
            0.516303s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of my-project @ file:[TEMP_PATH]/project (==0.1.0)
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
              0.516605s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/colorama/
       uv_resolver::version_map::from_metadata 
              0.516694s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/flask/
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=colorama==0.4.6
         uv_client::registry_client::wheel_metadata built_dist=colorama==0.4.6
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/colorama/colorama-0.4.6-py2.py3-none-any.msgpack
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=flask==3.0.0
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/colorama/colorama-0.4.6-py2.py3-none-any.msgpack"
         uv_client::registry_client::wheel_metadata built_dist=flask==3.0.0
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/flask/flask-3.0.0-py3-none-any.msgpack
            0.516952s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of colorama (>=0.4.6)
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/flask/flask-3.0.0-py3-none-any.msgpack"
            0.516980s   0ms DEBUG uv_resolver::resolver Selecting: colorama==0.4.6 (colorama-0.4.6-py2.py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=colorama, version=0.4.6
         uv_resolver::resolver::distributions_wait package_id=colorama-0.4.6
                  0.517022s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/d1/d6/3965ed04c63042e047cb6a3e6ed1a63a35087b6a609aa3a15ed8ac56c221/colorama-0.4.6-py2.py3-none-any.whl.metadata
       uv_resolver::resolver::choose_version package=flask
         uv_resolver::resolver::package_wait package_name=flask
            0.517072s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of flask (>=3.0.0)
            0.517079s   0ms DEBUG uv_resolver::resolver Selecting: flask==3.0.0 (flask-3.0.0-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=flask, version=3.0.0
         uv_resolver::resolver::distributions_wait package_id=flask-3.0.0
                  0.517107s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/36/42/015c23096649b908c809c69388a805a571a3bea44362fe87e33fc3afa01f/flask-3.0.0-py3-none-any.whl.metadata
            0.517137s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: werkzeug>=3.0.0
            0.517145s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: jinja2>=3.1.2
            0.517153s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: itsdangerous>=2.1.2
            0.517157s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: click>=8.1.3
            0.517161s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: blinker>=1.6.2
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
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/itsdangerous.rkyv"
         uv_client::cached_client::get_cacheable 
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/click.rkyv
     uv_resolver::resolver::process_request request=Versions blinker
       uv_client::registry_client::simple_api package=blinker
         uv_client::cached_client::get_cacheable 
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/click.rkyv"
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/blinker.rkyv
     uv_resolver::resolver::process_request request=Prefetch blinker >=1.6.2
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/blinker.rkyv"
     uv_resolver::resolver::process_request request=Prefetch click >=8.1.3
     uv_resolver::resolver::process_request request=Prefetch itsdangerous >=2.1.2
     uv_resolver::resolver::process_request request=Prefetch jinja2 >=3.1.2
     uv_resolver::resolver::process_request request=Prefetch werkzeug >=3.0.0
              0.517594s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/itsdangerous/
       uv_resolver::version_map::from_metadata 
              0.517645s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/jinja2/
       uv_resolver::version_map::from_metadata 
              0.517693s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/werkzeug/
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=itsdangerous==2.1.2
         uv_client::registry_client::wheel_metadata built_dist=itsdangerous==2.1.2
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/itsdangerous/itsdangerous-2.1.2-py3-none-any.msgpack
              0.517825s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/blinker/
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/itsdangerous/itsdangerous-2.1.2-py3-none-any.msgpack"
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=jinja2==3.1.2
         uv_client::registry_client::wheel_metadata built_dist=jinja2==3.1.2
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/jinja2/jinja2-3.1.2-py3-none-any.msgpack
              0.517930s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/click/
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/jinja2/jinja2-3.1.2-py3-none-any.msgpack"
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=werkzeug==3.0.1
         uv_client::registry_client::wheel_metadata built_dist=werkzeug==3.0.1
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/werkzeug/werkzeug-3.0.1-py3-none-any.msgpack
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=blinker==1.7.0
         uv_client::registry_client::wheel_metadata built_dist=blinker==1.7.0
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/blinker/blinker-1.7.0-py3-none-any.msgpack
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/werkzeug/werkzeug-3.0.1-py3-none-any.msgpack"
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/blinker/blinker-1.7.0-py3-none-any.msgpack"
                  0.518090s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/68/5f/447e04e828f47465eeab35b5d408b7ebaaaee207f48b7136c5a7267a30ae/itsdangerous-2.1.2-py3-none-any.whl.metadata
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=click==8.1.7
         uv_client::registry_client::wheel_metadata built_dist=click==8.1.7
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/click/click-8.1.7-py3-none-any.msgpack
                  0.518148s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/bc/c3/f068337a370801f372f2f8f6bad74a5c140f6fda3d9de154052708dd3c65/Jinja2-3.1.2-py3-none-any.whl.metadata
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/click/click-8.1.7-py3-none-any.msgpack"
            0.518183s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of werkzeug (>=3.0.0)
            0.518213s   1ms DEBUG uv_resolver::resolver Selecting: werkzeug==3.0.1 (werkzeug-3.0.1-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=werkzeug, version=3.0.1
         uv_resolver::resolver::distributions_wait package_id=werkzeug-3.0.1
                  0.518251s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/c3/fc/254c3e9b5feb89ff5b9076a23218dafbc99c96ac5941e900b71206e6313b/werkzeug-3.0.1-py3-none-any.whl.metadata
                  0.518272s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/fa/2a/7f3714cbc6356a0efec525ce7a0613d581072ed6eb53eb7b9754f33db807/blinker-1.7.0-py3-none-any.whl.metadata
                  0.518285s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/00/2e/d53fa4befbf2cfa713304affc7ca780ce4fc1fd8710527771b58311a3229/click-8.1.7-py3-none-any.whl.metadata
            0.518300s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: markupsafe>=2.1.1
       uv_resolver::resolver::choose_version package=jinja2
         uv_resolver::resolver::package_wait package_name=jinja2
            0.518323s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of jinja2 (>=3.1.2)
            0.518327s   0ms DEBUG uv_resolver::resolver Selecting: jinja2==3.1.2 (Jinja2-3.1.2-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=jinja2, version=3.1.2
         uv_resolver::resolver::distributions_wait package_id=jinja2-3.1.2
            0.518342s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: markupsafe>=2.0
       uv_resolver::resolver::choose_version package=itsdangerous
         uv_resolver::resolver::package_wait package_name=itsdangerous
            0.518356s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of itsdangerous (>=2.1.2)
            0.518362s   0ms DEBUG uv_resolver::resolver Selecting: itsdangerous==2.1.2 (itsdangerous-2.1.2-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=itsdangerous, version=2.1.2
         uv_resolver::resolver::distributions_wait package_id=itsdangerous-2.1.2
       uv_resolver::resolver::choose_version package=click
         uv_resolver::resolver::package_wait package_name=click
            0.518385s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of click (>=8.1.3)
            0.518390s   0ms DEBUG uv_resolver::resolver Selecting: click==8.1.7 (click-8.1.7-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=click, version=8.1.7
         uv_resolver::resolver::distributions_wait package_id=click-8.1.7
            0.518405s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: colorama*
       uv_resolver::resolver::choose_version package=blinker
         uv_resolver::resolver::package_wait package_name=blinker
            0.518421s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of blinker (>=1.6.2)
            0.518425s   0ms DEBUG uv_resolver::resolver Selecting: blinker==1.7.0 (blinker-1.7.0-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=blinker, version=1.7.0
         uv_resolver::resolver::distributions_wait package_id=blinker-1.7.0
       uv_resolver::resolver::choose_version package=markupsafe
         uv_resolver::resolver::package_wait package_name=markupsafe
     uv_resolver::resolver::process_request request=Versions markupsafe
       uv_client::registry_client::simple_api package=markupsafe
         uv_client::cached_client::get_cacheable 
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/markupsafe.rkyv
     uv_resolver::resolver::process_request request=Prefetch markupsafe >=2.1.1
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/markupsafe.rkyv"
              0.518990s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/markupsafe/
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=markupsafe==2.1.3
         uv_client::registry_client::wheel_metadata built_dist=markupsafe==2.1.3
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/markupsafe/markupsafe-2.1.3-cp312-cp312-win_amd64.msgpack
            0.519280s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of markupsafe (>=2.1.1)
            0.519286s   0ms DEBUG uv_resolver::resolver Selecting: markupsafe==2.1.3 (MarkupSafe-2.1.3-cp312-cp312-win_amd64.whl)
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/markupsafe/markupsafe-2.1.3-cp312-cp312-win_amd64.msgpack"
       uv_resolver::resolver::get_dependencies package=markupsafe, version=2.1.3
         uv_resolver::resolver::distributions_wait package_id=markupsafe-2.1.3
                  0.519417s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/44/44/dbaf65876e258facd65f586dde158387ab89963e7f2235551afc9c2e24c2/MarkupSafe-2.1.3-cp312-cp312-win_amd64.whl.metadata
    Resolved 9 packages in [EXECUTION_TIME]
     uv::requirements::from_source source=[TEMP_FILE]
     uv::requirements::from_source source=[TEMP_FILE]
     uv_interpreter::interpreter::find_best python_version=Some(PythonVersion(StringVersion { string: "3.12.2", version: "3.12.2" }))
          0.001779s   0ms DEBUG uv_interpreter::interpreter Starting interpreter discovery for Python 3.12.2
          0.001968s   0ms DEBUG uv_interpreter::python_environment Found a virtualenv named .venv at: [TEMP_PATH]/project/.venv
          0.002214s   0ms DEBUG uv_interpreter::interpreter Cached interpreter info for Python 3.12.2, skipping probing: [TEMP_PATH]/project/.venv/Scripts/python.exe
        0.002230s DEBUG uv::commands::pip_compile Using Python 3.12.2 interpreter at [36mC:[TEMP_PATH]/project/.venv/Scripts/python.exe[39m for builds
        0.002297s DEBUG uv_client::registry_client Using registry request timeout of 300s
     uv_client::flat_index::from_entries 
     uv_installer::downloader::build_editables 
          0.005538s   0ms DEBUG uv_distribution::source Building (editable) file:[TEMP_PATH]/project
       uv_dispatch::setup_build package_id="file:[TEMP_PATH]/project", subdirectory=None
         uv_resolver::resolver::solve 
              0.011927s   0ms DEBUG uv_resolver::resolver Solving with target Python version 3.12.2
           uv_resolver::resolver::choose_version package=root
           uv_resolver::resolver::get_dependencies package=root, version=0a0.dev0
                0.012014s   0ms DEBUG uv_resolver::resolver Adding direct dependency: hatchling*
           uv_resolver::resolver::choose_version package=hatchling
             uv_resolver::resolver::package_wait package_name=hatchling
         uv_resolver::resolver::process_request request=Versions hatchling
           uv_client::registry_client::simple_api package=hatchling
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/hatchling.rkyv
         uv_resolver::resolver::process_request request=Prefetch hatchling *
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/hatchling.rkyv"
                  0.012651s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/hatchling/
           uv_resolver::version_map::from_metadata 
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=hatchling==1.18.0
             uv_client::registry_client::wheel_metadata built_dist=hatchling==1.18.0
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/hatchling/hatchling-1.18.0-py3-none-any.msgpack
                0.012897s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of hatchling (*)
                0.012909s   0ms DEBUG uv_resolver::resolver Selecting: hatchling==1.18.0 (hatchling-1.18.0-py3-none-any.whl)
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/hatchling/hatchling-1.18.0-py3-none-any.msgpack"
           uv_resolver::resolver::get_dependencies package=hatchling, version=1.18.0
             uv_resolver::resolver::distributions_wait package_id=hatchling-1.18.0
                      0.013079s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/76/56/8ccca673e2c896931722f876bf040c0b6a7d8c1a128be60516a8a55bb27a/hatchling-1.18.0-py3-none-any.whl.metadata
                0.013160s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: editables>=0.3
                0.013169s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: packaging>=21.3
                0.013174s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: pathspec>=0.10.1
                0.013178s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: pluggy>=1.0.0
                0.013182s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: trove-classifiers*
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
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/pathspec.rkyv"
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/pluggy.rkyv
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/packaging.rkyv"
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
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/trove-classifiers.rkyv"
                  0.013645s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/editables/
           uv_resolver::version_map::from_metadata 
                  0.013689s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/pathspec/
           uv_resolver::version_map::from_metadata 
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=editables==0.5
             uv_client::registry_client::wheel_metadata built_dist=editables==0.5
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/editables/editables-0.5-py3-none-any.msgpack
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=pathspec==0.11.2
             uv_client::registry_client::wheel_metadata built_dist=pathspec==0.11.2
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/editables/editables-0.5-py3-none-any.msgpack"
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/pathspec/pathspec-0.11.2-py3-none-any.msgpack
                  0.013888s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/packaging/
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/pathspec/pathspec-0.11.2-py3-none-any.msgpack"
           uv_resolver::version_map::from_metadata 
                  0.013970s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/pluggy/
           uv_resolver::version_map::from_metadata 
                  0.014054s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/trove-classifiers/
           uv_resolver::version_map::from_metadata 
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=packaging==23.2
             uv_client::registry_client::wheel_metadata built_dist=packaging==23.2
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/packaging/packaging-23.2-py3-none-any.msgpack
                      0.014186s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/6b/be/0f2f4a5e8adc114a02b63d92bf8edbfa24db6fc602fca83c885af2479e0e/editables-0.5-py3-none-any.whl.metadata
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/packaging/packaging-23.2-py3-none-any.msgpack"
                      0.014211s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/b4/2a/9b1be29146139ef459188f5e420a66e835dda921208db600b7037093891f/pathspec-0.11.2-py3-none-any.whl.metadata
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=pluggy==1.3.0
             uv_client::registry_client::wheel_metadata built_dist=pluggy==1.3.0
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/pluggy/pluggy-1.3.0-py3-none-any.msgpack
           uv_distribution::distribution_database::get_or_build_wheel_metadata dist=trove-classifiers==2023.11.14
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/pluggy/pluggy-1.3.0-py3-none-any.msgpack"
             uv_client::registry_client::wheel_metadata built_dist=trove-classifiers==2023.11.14
               uv_client::cached_client::get_serde 
                 uv_client::cached_client::get_cacheable 
                   uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/trove-classifiers/trove_classifiers-2023.11.14-py3-none-any.msgpack
                      0.014369s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/ec/1a/610693ac4ee14fcdf2d9bf3c493370e4f2ef7ae2e19217d7a237ff42367d/packaging-23.2-py3-none-any.whl.metadata
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/trove-classifiers/trove_classifiers-2023.11.14-py3-none-any.msgpack"
                0.014393s   1ms DEBUG uv_resolver::resolver Searching for a compatible version of editables (>=0.3)
                0.014402s   1ms DEBUG uv_resolver::resolver Selecting: editables==0.5 (editables-0.5-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=editables, version=0.5
             uv_resolver::resolver::distributions_wait package_id=editables-0.5
           uv_resolver::resolver::choose_version package=packaging
             uv_resolver::resolver::package_wait package_name=packaging
                0.014441s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of packaging (>=21.3)
                0.014446s   0ms DEBUG uv_resolver::resolver Selecting: packaging==23.2 (packaging-23.2-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=packaging, version=23.2
             uv_resolver::resolver::distributions_wait package_id=packaging-23.2
           uv_resolver::resolver::choose_version package=pathspec
             uv_resolver::resolver::package_wait package_name=pathspec
                0.014474s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of pathspec (>=0.10.1)
                0.014477s   0ms DEBUG uv_resolver::resolver Selecting: pathspec==0.11.2 (pathspec-0.11.2-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=pathspec, version=0.11.2
             uv_resolver::resolver::distributions_wait package_id=pathspec-0.11.2
           uv_resolver::resolver::choose_version package=pluggy
             uv_resolver::resolver::package_wait package_name=pluggy
                0.014503s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of pluggy (>=1.0.0)
                0.014509s   0ms DEBUG uv_resolver::resolver Selecting: pluggy==1.3.0 (pluggy-1.3.0-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=pluggy, version=1.3.0
             uv_resolver::resolver::distributions_wait package_id=pluggy-1.3.0
                      0.014531s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/05/b8/42ed91898d4784546c5f06c60506400548db3f7a4b3fb441cba4e5c17952/pluggy-1.3.0-py3-none-any.whl.metadata
                      0.014554s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/a9/58/3feea94f12f25714f54a1cc14f3760977631d62c70952de3ab4bd0c6bc41/trove_classifiers-2023.11.14-py3-none-any.whl.metadata
           uv_resolver::resolver::choose_version package=trove-classifiers
             uv_resolver::resolver::package_wait package_name=trove-classifiers
                0.014581s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of trove-classifiers (*)
                0.014586s   0ms DEBUG uv_resolver::resolver Selecting: trove-classifiers==2023.11.14 (trove_classifiers-2023.11.14-py3-none-any.whl)
           uv_resolver::resolver::get_dependencies package=trove-classifiers, version=2023.11.14
             uv_resolver::resolver::distributions_wait package_id=trove-classifiers-2023.11.14
         uv_dispatch::install resolution="editables==0.5, trove-classifiers==2023.11.14, pathspec==0.11.2, pluggy==1.3.0, packaging==23.2, hatchling==1.18.0", venv="[TEMP_PATH]//uv-cache/.tmpIRxWnH/.venv"
              0.014697s   0ms DEBUG uv_dispatch Installing in editables==0.5, trove-classifiers==2023.11.14, pathspec==0.11.2, pluggy==1.3.0, packaging==23.2, hatchling==1.18.0 in [TEMP_PATH]/uv-cache/.tmpIRxWnH/.venv
              0.015090s   0ms DEBUG uv_installer::plan Requirement already cached: editables==0.5
              0.015393s   0ms DEBUG uv_installer::plan Requirement already cached: hatchling==1.18.0
              0.015592s   0ms DEBUG uv_installer::plan Requirement already cached: packaging==23.2
              0.015888s   1ms DEBUG uv_installer::plan Requirement already cached: pathspec==0.11.2
              0.016149s   1ms DEBUG uv_installer::plan Requirement already cached: pluggy==1.3.0
              0.016489s   1ms DEBUG uv_installer::plan Requirement already cached: trove-classifiers==2023.11.14
              0.016518s   1ms DEBUG uv_dispatch Installing build requirements: editables==0.5, hatchling==1.18.0, packaging==23.2, pathspec==0.11.2, pluggy==1.3.0, trove-classifiers==2023.11.14
           uv_installer::installer::install num_wheels=6
            0.040770s  35ms DEBUG uv_build Calling `hatchling.build.get_requires_for_build_editable()`
         uv_build::run_python_script script="get_requires_for_build_editable", python_version=3.12.2
       uv_build::build package_id="file:[TEMP_PATH]/project"
            0.268097s   0ms DEBUG uv_build Calling `hatchling.build.build_editable(metadata_directory=None)`
         uv_build::run_python_script script="build_editable", python_version=3.12.2
          0.515821s 510ms DEBUG uv_distribution::source Finished building (editable): my-project @ file:[TEMP_PATH]/project
     uv_distribution::unzip::unzip filename="my_project-0.1.0-py3-none-any.whl"
    Built 1 editable in [EXECUTION_TIME]
     uv_resolver::resolver::solve 
          0.519386s   0ms DEBUG uv_resolver::resolver Solving with target Python version 3.12.2
       uv_resolver::resolver::choose_version package=root
       uv_resolver::resolver::get_dependencies package=root, version=0a0.dev0
       uv_resolver::resolver::choose_version package=my-project[web]
            0.519475s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of my-project[web] @ file:[TEMP_PATH]/project (==0.1.0)
       uv_resolver::resolver::get_dependencies package=my-project[web], version=0.1.0
            0.519494s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: colorama>=0.4.6
            0.519501s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: flask>=3.0.0
       uv_resolver::resolver::choose_version package=my-project
            0.519526s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of my-project @ file:[TEMP_PATH]/project (==0.1.0)
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
              0.519811s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/colorama/
       uv_resolver::version_map::from_metadata 
              0.519874s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/flask/
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=colorama==0.4.6
         uv_client::registry_client::wheel_metadata built_dist=colorama==0.4.6
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/colorama/colorama-0.4.6-py2.py3-none-any.msgpack
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=flask==3.0.0
         uv_client::registry_client::wheel_metadata built_dist=flask==3.0.0
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/colorama/colorama-0.4.6-py2.py3-none-any.msgpack"
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/flask/flask-3.0.0-py3-none-any.msgpack
            0.520031s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of colorama (>=0.4.6)
            0.520039s   0ms DEBUG uv_resolver::resolver Selecting: colorama==0.4.6 (colorama-0.4.6-py2.py3-none-any.whl)
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/flask/flask-3.0.0-py3-none-any.msgpack"
       uv_resolver::resolver::get_dependencies package=colorama, version=0.4.6
         uv_resolver::resolver::distributions_wait package_id=colorama-0.4.6
                  0.520130s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/d1/d6/3965ed04c63042e047cb6a3e6ed1a63a35087b6a609aa3a15ed8ac56c221/colorama-0.4.6-py2.py3-none-any.whl.metadata
       uv_resolver::resolver::choose_version package=flask
         uv_resolver::resolver::package_wait package_name=flask
            0.520170s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of flask (>=3.0.0)
            0.520174s   0ms DEBUG uv_resolver::resolver Selecting: flask==3.0.0 (flask-3.0.0-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=flask, version=3.0.0
         uv_resolver::resolver::distributions_wait package_id=flask-3.0.0
                  0.520194s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/36/42/015c23096649b908c809c69388a805a571a3bea44362fe87e33fc3afa01f/flask-3.0.0-py3-none-any.whl.metadata
            0.520215s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: werkzeug>=3.0.0
            0.520223s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: jinja2>=3.1.2
            0.520227s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: itsdangerous>=2.1.2
            0.520230s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: click>=8.1.3
            0.520235s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: blinker>=1.6.2
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
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/itsdangerous.rkyv"
         uv_client::cached_client::get_cacheable 
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
              0.520605s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/werkzeug/
       uv_resolver::version_map::from_metadata 
              0.520692s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/jinja2/
       uv_resolver::version_map::from_metadata 
              0.520740s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/itsdangerous/
       uv_resolver::version_map::from_metadata 
              0.520776s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/click/
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=werkzeug==3.0.1
         uv_client::registry_client::wheel_metadata built_dist=werkzeug==3.0.1
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/werkzeug/werkzeug-3.0.1-py3-none-any.msgpack
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=jinja2==3.1.2
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/werkzeug/werkzeug-3.0.1-py3-none-any.msgpack"
         uv_client::registry_client::wheel_metadata built_dist=jinja2==3.1.2
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/jinja2/jinja2-3.1.2-py3-none-any.msgpack
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/jinja2/jinja2-3.1.2-py3-none-any.msgpack"
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=itsdangerous==2.1.2
         uv_client::registry_client::wheel_metadata built_dist=itsdangerous==2.1.2
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/itsdangerous/itsdangerous-2.1.2-py3-none-any.msgpack
              0.520999s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/blinker/
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/itsdangerous/itsdangerous-2.1.2-py3-none-any.msgpack"
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=click==8.1.7
         uv_client::registry_client::wheel_metadata built_dist=click==8.1.7
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/click/click-8.1.7-py3-none-any.msgpack
                  0.521068s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/c3/fc/254c3e9b5feb89ff5b9076a23218dafbc99c96ac5941e900b71206e6313b/werkzeug-3.0.1-py3-none-any.whl.metadata
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/click/click-8.1.7-py3-none-any.msgpack"
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=blinker==1.7.0
         uv_client::registry_client::wheel_metadata built_dist=blinker==1.7.0
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/blinker/blinker-1.7.0-py3-none-any.msgpack
                  0.521126s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/bc/c3/f068337a370801f372f2f8f6bad74a5c140f6fda3d9de154052708dd3c65/Jinja2-3.1.2-py3-none-any.whl.metadata
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/blinker/blinker-1.7.0-py3-none-any.msgpack"
                  0.521159s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/68/5f/447e04e828f47465eeab35b5d408b7ebaaaee207f48b7136c5a7267a30ae/itsdangerous-2.1.2-py3-none-any.whl.metadata
            0.521173s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of werkzeug (>=3.0.0)
            0.521180s   0ms DEBUG uv_resolver::resolver Selecting: werkzeug==3.0.1 (werkzeug-3.0.1-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=werkzeug, version=3.0.1
         uv_resolver::resolver::distributions_wait package_id=werkzeug-3.0.1
            0.521199s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: markupsafe>=2.1.1
       uv_resolver::resolver::choose_version package=jinja2
         uv_resolver::resolver::package_wait package_name=jinja2
            0.521219s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of jinja2 (>=3.1.2)
            0.521223s   0ms DEBUG uv_resolver::resolver Selecting: jinja2==3.1.2 (Jinja2-3.1.2-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=jinja2, version=3.1.2
         uv_resolver::resolver::distributions_wait package_id=jinja2-3.1.2
            0.521239s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: markupsafe>=2.0
       uv_resolver::resolver::choose_version package=itsdangerous
         uv_resolver::resolver::package_wait package_name=itsdangerous
            0.521254s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of itsdangerous (>=2.1.2)
            0.521257s   0ms DEBUG uv_resolver::resolver Selecting: itsdangerous==2.1.2 (itsdangerous-2.1.2-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=itsdangerous, version=2.1.2
         uv_resolver::resolver::distributions_wait package_id=itsdangerous-2.1.2
       uv_resolver::resolver::choose_version package=click
         uv_resolver::resolver::package_wait package_name=click
            0.521279s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of click (>=8.1.3)
            0.521284s   0ms DEBUG uv_resolver::resolver Selecting: click==8.1.7 (click-8.1.7-py3-none-any.whl)
       uv_resolver::resolver::get_dependencies package=click, version=8.1.7
         uv_resolver::resolver::distributions_wait package_id=click-8.1.7
                  0.521301s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/00/2e/d53fa4befbf2cfa713304affc7ca780ce4fc1fd8710527771b58311a3229/click-8.1.7-py3-none-any.whl.metadata
                  0.521314s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/fa/2a/7f3714cbc6356a0efec525ce7a0613d581072ed6eb53eb7b9754f33db807/blinker-1.7.0-py3-none-any.whl.metadata
     uv_resolver::resolver::process_request request=Versions markupsafe
       uv_client::registry_client::simple_api package=markupsafe
         uv_client::cached_client::get_cacheable 
           uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/simple-v3/b2a7eb67d4c26b82/markupsafe.rkyv
     uv_resolver::resolver::process_request request=Prefetch markupsafe >=2.1.1
            0.521355s   0ms DEBUG uv_resolver::resolver Adding transitive dependency: colorama*
       uv_resolver::resolver::choose_version package=blinker
         uv_resolver::resolver::package_wait package_name=blinker
            0.521371s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of blinker (>=1.6.2)
            0.521375s   0ms DEBUG uv_resolver::resolver Selecting: blinker==1.7.0 (blinker-1.7.0-py3-none-any.whl)
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/simple-v3/b2a7eb67d4c26b82/markupsafe.rkyv"
       uv_resolver::resolver::get_dependencies package=blinker, version=1.7.0
         uv_resolver::resolver::distributions_wait package_id=blinker-1.7.0
       uv_resolver::resolver::choose_version package=markupsafe
         uv_resolver::resolver::package_wait package_name=markupsafe
              0.521948s   0ms DEBUG uv_client::cached_client Found fresh response for: https://pypi.org/simple/markupsafe/
       uv_resolver::version_map::from_metadata 
       uv_distribution::distribution_database::get_or_build_wheel_metadata dist=markupsafe==2.1.3
         uv_client::registry_client::wheel_metadata built_dist=markupsafe==2.1.3
           uv_client::cached_client::get_serde 
             uv_client::cached_client::get_cacheable 
               uv_client::cached_client::read_and_parse_cache file=[TEMP_PATH]/uv-cache/wheels-v0/index/b2a7eb67d4c26b82/markupsafe/markupsafe-2.1.3-cp312-cp312-win_amd64.msgpack
            0.522228s   0ms DEBUG uv_resolver::resolver Searching for a compatible version of markupsafe (>=2.1.1)
            0.522234s   0ms DEBUG uv_resolver::resolver Selecting: markupsafe==2.1.3 (MarkupSafe-2.1.3-cp312-cp312-win_amd64.whl)
     uv_client::cached_client::from_path_sync path="[TEMP_PATH]//uv-cache/wheels-v0/index/b2a7eb67d4c26b82/markupsafe/markupsafe-2.1.3-cp312-cp312-win_amd64.msgpack"
       uv_resolver::resolver::get_dependencies package=markupsafe, version=2.1.3
         uv_resolver::resolver::distributions_wait package_id=markupsafe-2.1.3
                  0.522364s   0ms DEBUG uv_client::cached_client Found fresh response for: https://files.pythonhosted.org/packages/44/44/dbaf65876e258facd65f586dde158387ab89963e7f2235551afc9c2e24c2/MarkupSafe-2.1.3-cp312-cp312-win_amd64.whl.metadata
    Resolved 9 packages in [EXECUTION_TIME]
     uv::requirements::from_source source=[TEMP_PATH]/project/requirements-dev.lock
        0.000790s DEBUG uv_interpreter::python_environment Found a virtualenv through VIRTUAL_ENV at: [TEMP_PATH]/project/.venv
        0.001145s DEBUG uv_interpreter::interpreter Cached interpreter info for Python 3.12.2, skipping probing: [TEMP_PATH]/project/.venv/Scripts/python.exe
        0.001162s DEBUG uv::commands::pip_sync Using Python 3.12.2 environment at [36mC:[TEMP_PATH]/project/.venv/Scripts/python.exe[39m
        0.001331s DEBUG uv_client::registry_client Using registry request timeout of 300s
     uv_client::flat_index::from_entries 
        0.005682s DEBUG uv_installer::plan Treating editable requirement as immutable: my-project==0.1.0 (from file:[TEMP_PATH]/project)
        0.005701s DEBUG uv_installer::plan Requirement already satisfied: blinker==1.7.0
        0.005707s DEBUG uv_installer::plan Requirement already satisfied: click==8.1.7
        0.005710s DEBUG uv_installer::plan Requirement already satisfied: colorama==0.4.6
        0.005713s DEBUG uv_installer::plan Requirement already satisfied: flask==3.0.0
        0.005718s DEBUG uv_installer::plan Requirement already satisfied: itsdangerous==2.1.2
        0.005721s DEBUG uv_installer::plan Requirement already satisfied: jinja2==3.1.2
        0.005723s DEBUG uv_installer::plan Requirement already satisfied: markupsafe==2.1.3
        0.005727s DEBUG uv_installer::plan Requirement already satisfied: werkzeug==3.0.1
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
