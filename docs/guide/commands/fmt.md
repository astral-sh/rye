# `fmt`

+++ 0.20.0

Run the code formatter on the project.  This command is aliased to `format`.

For more information about how to configure Ruff, have a look at the
[Ruff Configuration Documentation](https://docs.astral.sh/ruff/configuration/).

## Example

To format the code and write back to the files:

```
$ rye fmt
1 file reformatted, 231 files left unchanged
```

To just check if the code needs formatting:

```
$ rye fmt --check
Would reformat: src/my_project/utils.py
1 file would be reformatted, 231 files already formatted
```

To pass extra arguments to the underlying `ruff` formatter use `--`:

```
$ rye fmt -- --diff
--- src/my_project/utils.py
+++ src/my_project/utils.py
@@ -2,5 +2,4 @@


 def foo():
-
     pass

1 file would be reformatted, 231 files already formatted
```

Format a specific file:

```
rye fmt src/foo.py
```

## Arguments

* `[PATHS]...` List of files or directories to lint.  If not supplied all files are formatted.

* `[EXTRA_ARGS]...` Extra arguments to the formatter.

    These arguments are forwarded directly to the underlying formatter (currently
    always `ruff`).  Note that extra arguments must be separated from other arguments
    with the `--` marker.

## Options

* `-a, --all`: Format all packages in the workspace

* `-p, --package <PACKAGE>`: Format a specific package

* `--pyproject <PYPROJECT_TOML>`: Use this `pyproject.toml` file

* `--check`: Run format in check mode

* `-v, --verbose`: Enables verbose diagnostics

* `-q, --quiet`: Turns off all output

* `-h, --help`: Print help (see a summary with '-h')