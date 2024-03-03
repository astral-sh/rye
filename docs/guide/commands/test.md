# `test`

+++ X.X.X

Run the test suites of the project. At the moment this always runs `pytest`.

For more information about how to use pytest, have a look at the
[Pytest Documentation](https://docs.pytest.org/en/8.0.x/).

## Example

Run the test suite:

```
$ rye test
platform win32 -- Python 3.11.1, pytest-8.0.2, pluggy-1.4.0
rootdir: C:\Users\User\rye
plugins: anyio-4.3.0
collected 1 item

rye-devtools\tests\test_batch.py .                                            [100%] 
```

## Arguments

* `[EXTRA_ARGS]...` Extra arguments to the test runner.

    These arguments are forwarded directly to the underlying test runner (currently
    always `pytest`).  Note that extra arguments must be separated from other arguments
    with the `--` marker.

## Options

* `-a, --all`: Lint all packages in the workspace

* `-p, --package <PACKAGE>`: Run the test suite of a specific package

* `--pyproject <PYPROJECT_TOML>`: Use this `pyproject.toml` file

* `-v, --verbose`: Enables verbose diagnostics

* `-q, --quiet`: Turns off all output

* `-i, --ignore`: Ignore the specified directory

* `--no-capture`: Disable stdout/stderr capture for the test runner

* `-h, --help`: Print help (see a summary with '-h')