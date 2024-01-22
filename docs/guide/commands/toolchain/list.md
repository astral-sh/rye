# `list`

List all registered toolchains.  It can list the toolchains which are installed as
well as toolchains which can be downloaded if `--include-downloadable` is passed.

## Example

List installed toolchains:

```
$ rye toolchain list
cpython@3.12.1 (/Users/username/.rye/py/cpython@3.12.1/install/bin/python3)
cpython@3.11.6 (/Users/username/.rye/py/cpython@3.11.6/install/bin/python3)
```

Lists downloadable toolchains:

```
$ rye toolchain list --include-downloadable
cpython@3.12.1 (/Users/mitsuhiko/.rye/py/cpython@3.12.1/install/bin/python3)
cpython-x86_64@3.12.1 (downloadable)
cpython@3.11.7 (downloadable)
...
```

## Arguments

*no arguments*

## Options

* `--include-downloadable`: Also include non installed, but downloadable toolchains

* `--format <FORMAT>`: Request parseable output format [possible values: json]

* `-h, --help`: Print help
