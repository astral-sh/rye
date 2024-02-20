# `remove`

Removes a package from this project.  This removes a package from the `pyproject.toml`
dependency list.

If auto sync is disabled, after a dependency is removed it's not automatically
uninstalled.  To do that, you need to invoke the [`sync`](sync.md) command or pass
`--sync`.

+++ 0.26.0

    Added support for auto-sync and the `--sync` / `--no-sync` flags.

## Example

```
$ rye remove flask
Removed flask>=3.0.1
```

## Arguments

* `<REQUIREMENTS>...`: The packages to remove from the project

## Options

* `--dev`: Remove this from dev dependencies

* `--optional <OPTIONAL>`: Remove this from the optional dependency group

* `--sync`: Runs `sync` automatically even if auto-sync is disabled.

* `--no-sync`: Does not run `sync` automatically even if auto-sync is enabled.

* `-v, --verbose`: Enables verbose diagnostics

* `-q, --quiet`: Turns off all output

* `-h, --help`: Print help (see a summary with '-h')