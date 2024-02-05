# `pin`

Pins a Python version to this project.

This will update the `.python-version` to point to the provided version.
Additionally it will update `requires-python` in the `pyproject.toml` if it's
lower than the current version.  This can be disabled by passing
`--no-update-requires-python`.

Which toolchain Rye prefers depends on the Rye version.  From 0.22 onwards
the latest compatible installed toolchain is picked, and only if a non
existing one is found a download will be attempted.  For older versions
Rye will always attempt to download the latest available if it's not
installed yet unless a precise pin is selected.

## Example

Pin a specific version of Python:

```
$ rye pin 3.9
pinned 3.9.18 in /Users/username/my-project
```

To issue a relaxed and not a specific pin use `--relaxed`:

```
$ rye pin 3.9 --relaxed
pinned 3.9 in /Users/username/my-project
```

## Arguments

* `<VERSION>`: The version of Python to pin

    This can be a short version (3.9) or a full one (`cpython@3.9.18`).

## Options

* `--relaxed`: Issue a relaxed pin

* `--no-update-requires-python`: Prevent updating requires-python in the `pyproject.toml`

* `--pyproject <PYPROJECT_TOML>`: Use this `pyproject.toml` file

* `-h, --help`: Print help (see a summary with '-h')