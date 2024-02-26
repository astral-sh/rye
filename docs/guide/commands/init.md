# `init`

This command initializes a new or existing Python project with Rye.  Running it in
a folder with an already existing Python project will attempt to convert it over
and bootstrap Rye right there.  Otherwise it can be used to create a completely new
project from scratch.

For more information see the [Basics Guide](../basics.md).

## Example

```
$ rye init
success: Initialized project in /Users/john/Development/my-project.
  Run `rye sync` to get started
```

## Arguments

* `[PATH]`: Where to place the project (defaults to current path)

## Options

* `--min-py <MIN_PY>`: Minimal Python version supported by this project

* `-p, --py <PY>`: Python version to use for the virtualenv

* `--no-readme`: Do not create a readme

* `--no-pin`: Do not create .python-version file (requires-python will be used)

* `--build-system <BUILD_SYSTEM>`: Which build system should be used(defaults to hatchling)?

    [possible values: `hatchling`, `setuptools`, `flit`, `pdm`, `maturin`]

* `--license <LICENSE>`: Which license should be used? [SPDX identifier](https://spdx.org/licenses/)

* `--name <NAME>`: The name of the package

* `--private`: Set "Private :: Do Not Upload" classifier, used for private projects

* `--no-import`: Don't import from setup.cfg, setup.py, or requirements files

* `--virtual`: Initialize this as a virtual package.

    A virtual package can have dependencies but is itself not installed as a Python package.  It also cannot be published.

* `-r, --requirements <REQUIREMENTS_FILE>`: Requirements files to initialize pyproject.toml with

* `--dev-requirements <DEV_REQUIREMENTS_FILE>`: Development requirements files to initialize pyproject.toml with

* `-v, --verbose`: Enables verbose diagnostics

* `-q, --quiet`: Turns off all output

* `-h, --help`: Print help (see a summary with '-h')
