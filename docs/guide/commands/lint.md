# `lint`

+++ 0.20.0

Run the linter on the project.  This command is aliased to `check`.  At the moment
this always runs `ruff` in lint mode.

For more information about how to configure Ruff, have a look at the
[Ruff Configuration Documentation](https://docs.astral.sh/ruff/configuration/).

## Example

Run the linter:

```
$ rye lint
src/myproject/sdk.py:1:8: F401 [*] `sys` imported but unused
Found 1 error.
[*] 1 fixable with the `--fix` option.
```

For issues that can be auto fixed pass `--fix`:

```
$ rye lint --fix
Found 1 error (1 fixed, 0 remaining).
```

To pass extra arguments:

```
$ rye lint -- --watch
```

Lint a specific file:

```
rye lint src/foo.py
```

## Arguments

* `[PATHS]...` List of files or directories to lint.  If not supplied all files are linted.

* `[EXTRA_ARGS]...` Extra arguments to the linter.

    These arguments are forwarded directly to the underlying linter (currently
    always `ruff`).  Note that extra arguments must be separated from other arguments
    with the `--` marker.

## Options

* `-a, --all`: Lint all packages in the workspace

* `-p, --package <PACKAGE>`: Format a specific package

* `--pyproject <PYPROJECT_TOML>`: Use this `pyproject.toml` file

* `--fix`: Automatically fix fixable issues

* `-v, --verbose`: Enables verbose diagnostics

* `-q, --quiet`: Turns off all output

* `-h, --help`: Print help (see a summary with '-h')