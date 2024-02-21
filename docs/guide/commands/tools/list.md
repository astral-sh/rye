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
$ rye tools list --include-version
pycowsay 0.0.0.2 (cpython@3.12.1)
```

## Arguments

*no arguments*

## Options

* `-s, --include-scripts`: Show all the scripts installed by the tools

+/- 0.26.0

    Renamed from `-i, --include-scripts` to `-s, --include-scripts`.

* `-v, --include-version`: Show the version of tools

+/- 0.26.0

    Renamed from `-v, --version-show` to `-v, --include-version`.

* `-h, --help`: Print help
