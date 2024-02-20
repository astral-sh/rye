# `list`

Lists all already installed global tools.

For more information see [Tools](/guide/tools/).

## Example

List installed tools:

```
$ rye tools list
pycowsay
```

List installed tools with version:

```
$ rye tools list --version-show
pycowsay 0.0.0.2 (cpython@3.12.1)
```

## Arguments

*no arguments*

## Options

* `-i, --include-scripts`: Also show all the scripts installed by the tools

* `-v, --version-show`: Show the version of tools

* `-h, --help`: Print help
