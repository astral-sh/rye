# `test`

+++ 0.28.0

Run the test suites of the project. At the moment this always runs `pytest`.
Note that `pytest` must be installed into the virtual env unlike `ruff`
which is used behind the scenes automatically for linting and formatting.
Thus in order to use this, you need to declare `pytest` as dev dependency.

```
$ rye add --dev pytest
```

It's recommended to place tests in a folder called `tests` adjacent to the
`src` folder of your project.

For more information about how to use pytest, have a look at the
[Pytest Documentation](https://docs.pytest.org/en/8.0.x/).

## Example

Run the test suite:

```
$ rye test
platform win32 -- Python 3.11.1, pytest-8.0.2, pluggy-1.4.0
rootdir: /Users/john/Development/stuff
plugins: anyio-4.3.0
collected 1 item

stuff/tests/test_batch.py .                                            [100%] 
```

## Arguments

* `[EXTRA_ARGS]...` Extra arguments to the test runner.

    These arguments are forwarded directly to the underlying test runner (currently
    always `pytest`).  Note that extra arguments must be separated from other arguments
    with the `--` marker.

## Options

* `-a, --all`: Test all packages in the workspace

* `-p, --package <PACKAGE>`: Run the test suite of a specific package

* `--pyproject <PYPROJECT_TOML>`: Use this `pyproject.toml` file

* `-v, --verbose`: Enables verbose diagnostics

* `-q, --quiet`: Turns off all output

* `-i, --ignore`: Ignore the specified directory

* `-s`, `--no-capture`: Disable stdout/stderr capture for the test runner

* `-h, --help`: Print help (see a summary with '-h')
