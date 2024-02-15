# `show`

Prints the current state of the project.  This can print out information about the
virtualenv, the project or workspace as well as a list of installed dependencies.

## Example

Print out the status of a project:

```
$ rye show
project: my-project
path: /Users/username/my-project
venv: /Users/username/my-project/.venv
target python: 3.8
venv python: cpython@3.9.18
virtual: false
```

## Arguments

*no arguments*

## Options

* `--installed-deps`: Print the currently installed dependencies.

    This option is being replaced with [`rye list`](list.md)

* `--pyproject`: Use this `pyproject.toml` file

* `-h, --help`: Print help (see a summary with '-h')