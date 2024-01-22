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

To print out the list of installed dependencies:

```
$ rye show --installed-deps
asgiref==3.7.2
blinker==1.7.0
click==8.1.7
Flask @ git+https://github.com/pallets/flask@4df377cfbfc1d15e962a61c18920b22aebc9aa41
itsdangerous==2.1.2
Jinja2==3.1.3
MarkupSafe==2.1.4
Werkzeug==3.0.1
```

## Arguments

*no arguments*

## Options

* `--installed-deps`: Print the currently installed dependencies

* `--pyproject`: Use this `pyproject.toml` file

* `-h, --help`: Print help (see a summary with '-h')