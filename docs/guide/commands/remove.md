# `remove`

Removes a package from this project.  This removes a package from the `pyproject.toml`
dependency list.

## Example

```
$ rye remove flask
Removed flask>=3.0.1
```

## Arguments

* `<REQUIREMENTS>...`: The packages to remove from the project

## Options

* `--dev`: Remove this from dev dependencies

* `--optional <OPTIONAL>`: Remove this from the optional dependency group

* `-v, --verbose`: Enables verbose diagnostics

* `-q, --quiet`: Turns off all output

* `-h, --help`: Print help (see a summary with '-h')