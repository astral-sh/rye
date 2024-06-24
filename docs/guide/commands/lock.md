# `lock`

Updates the lockfiles without installing dependencies.  Usually one would use
the [`sync`](sync.md) command instead which both locks and installs dependencies.

For more information see [Syncing and Locking](../sync.md).

## Example

```
$ rye lock
Generating production lockfile: /Users/username/my-project/requirements.lock
Generating dev lockfile: /Users/username/my-project/requirements-dev.lock
Done!
```

## Arguments

*no arguments*

## Options

* `--update <UPDATE>`: Update a specific package

* `--update-all`: Update all packages to the latest

* `--pre`: Update to pre-release versions

* `--features <FEATURES>`: Extras/features to enable when locking the workspace

* `--all-features`: Enables all features

* `--generate-hashes`: Set to true to lock with hashes in the lockfile

* `--with-sources`: Set to true to lock with sources in the lockfile

* `--pyproject <PYPROJECT_TOML>`: Use this pyproject.toml file

* `-v, --verbose`: Enables verbose diagnostics

* `-q, --quiet`: Turns off all output

* `-h, --help`: Print help (see a summary with '-h')