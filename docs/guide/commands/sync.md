# `sync`

Updates the lockfiles and installs the dependencies into the virtualenv.

For more information see [Syncing and Locking](../sync.md).

## Example

Sync the project:

```
$ rye sync
Reusing already existing virtualenv
Generating production lockfile: /Users/username/my-project/requirements.lock
Generating dev lockfile: /Users/username/my-project/requirements-dev.lock
Installing dependencies
...
```

To sync without updating the lock file use `--no-lock`:

```
$ rye sync --no-lock
```

If you do not want dev dependencies to be installed use `--no-dev`:

```
$ rye sync --no-dev
```

To exit the sub shell run `exit`.

## Arguments

*no arguments*

## Options

* `-f, --force`: Force the virtualenv to be re-created

* `--no-dev`: Do not install dev dependencies

* `--no-lock`: Do not update the lockfile.

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
