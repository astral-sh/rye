# `remove`

Removes or uninstalls a toolchain.

## Example

```
$ rye toolchain remove 3.9.5
Removed installed toolchain cpython@3.9.5
```

## Arguments

* `<VERSION>` The version of Python to remove.

## Options

* `-f, --force`: Force removal even if the toolchain is in use
* `-h, --help`: Print help (see a summary with '-h')