use crate::common::{rye_cmd_snapshot, Space};

mod common;

#[test]
fn test_config_empty() {
    let space = Space::new();
    rye_cmd_snapshot!(space.rye_cmd().arg("config"), @r###"
      success: false
      exit_code: 2
      ----- stdout -----

      ----- stderr -----
      Reads or modifies the global `config.toml` file

      Usage: rye config [OPTIONS]

      Options:
            --show-path            Print the path to the config
            --format <FORMAT>      Request parseable output format rather than lines [possible values:
                                   json]
            --get <GET>            Reads a config key
            --set <SET>            Sets a config key to a string
            --set-int <SET_INT>    Sets a config key to an integer
            --set-bool <SET_BOOL>  Sets a config key to a bool
            --unset <UNSET>        Remove a config key
        -h, --help                 Print help (see more with '--help')
    "###);
}

#[test]
fn test_config_show_path() {
    let space = Space::new();
    rye_cmd_snapshot!(space.rye_cmd().arg("config").arg("--show-path"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    [RYE_HOME]/config.toml

    ----- stderr -----
    "###);
}

#[test]
fn test_config_incompatible_format_and_show_path() {
    let space = Space::new();
    rye_cmd_snapshot!(space.rye_cmd().arg("config").arg("--show-path").arg("--format=json"), @r###"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: an argument cannot be used with one or more of the other specified arguments
    "###);
}

#[test]
fn test_config_get_set_multiple() {
    let space = Space::new();
    rye_cmd_snapshot!(space.rye_cmd()
        .arg("config")
        .arg("--set")
        .arg("default.toolchain=cpython@3.12")
        .arg("--set-bool")
        .arg("behavior.use-uv=true"),
    @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    "###);

    rye_cmd_snapshot!(space.rye_cmd()
        .arg("config")
        .arg("--get")
        .arg("default.toolchain")
        .arg("--get")
        .arg("behavior.use-uv")
        .arg("--format=json"),
    @r###"
    success: true
    exit_code: 0
    ----- stdout -----
    {
      "behavior.use-uv": true,
      "default.toolchain": "cpython@3.12"
    }

    ----- stderr -----
    "###);
}

#[test]
// This test ensure that --show-path is not compatible with any other action
fn test_config_show_path_and_any_action() {
    let space = Space::new();
    rye_cmd_snapshot!(space.rye_cmd()
        .arg("config")
        .arg("--set")
        .arg("default.toolchain=cpython@3.12")
        .arg("--show-path"),
    @r###"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: an argument cannot be used with one or more of the other specified arguments
    "###);
}
