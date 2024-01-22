# `config`

Reads or modifies the global `config.toml` file.

The config file can be read via `--get` and it can be set with one of the set options (`--set`, `--set-int`,
`--set-bool`, or `--unset`). Each of the set operations takes a key=value pair. All of these can be supplied
multiple times.

## Example

This command turns on global shims:

```
rye config --set-bool behavior.global-python=true
```

Reads the currently set config value for global Python shims:

```
$ rye config --get behavior.global-python
true
```

Show the path to the config:

```
$ rye config --show-path
/Users/username/.rye/config.toml
```

## Arguments

*no arguments*

## Options

* `--get <GET>`: Reads a config key

* `--set <SET>`: Sets a config key to a string

* `--set-int <SET_INT>`: Sets a config key to an integer

* `--set-bool <SET_BOOL>`: Sets a config key to a bool

* `--unset <UNSET>`: Remove a config key

* `--show-path`: Print the path to the config

* `--format <FORMAT>`: Request parseable output format rather than lines

    [possible values: json]

* `-h, --help`: Print help (see a summary with '-h')