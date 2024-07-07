# `build`

Builds a package for distribution.

Under normal circumstances Rye automatically builds packages for
local development.  However if you want to publish packages you need
to first build them into source distributions (`sdist`) and
binary/portable distributions (`wheel`).

For more information see [Building and Publishing](../publish.md).

## Example

This builds wheels and source distributions at once:

```
$ rye build
building my-project
* Creating virtualenv isolated environment...
* Installing packages in isolated environment... (hatchling)
* Getting build dependencies for sdist...
* Building sdist...
* Building wheel from sdist
* Creating virtualenv isolated environment...
* Installing packages in isolated environment... (hatchling)
* Getting build dependencies for wheel...
* Building wheel...
Successfully built my_project-0.1.0.tar.gz and my_project-0.1.0-py3-none-any.whl
```

By default you will find the artifacts in the `dist` folder.

## Arguments

*no arguments*

## Options

* `--sdist`: Build an sdist

* `--wheel`: Build a wheel

* `-a, --all`: Build all packages

* `-p, --package <PACKAGE>`: Build a specific package

* `-o, --out <OUT>`: An output directory (defaults to `workspace/dist`)

* `--pyproject <PYPROJECT_TOML>`: Use this `pyproject.toml` file

* `-c, --clean`: Clean the output directory first

* `-v, --verbose`: Enables verbose diagnostics

* `-q, --quiet`: Turns off all output

* `-h, --help`: Print help
