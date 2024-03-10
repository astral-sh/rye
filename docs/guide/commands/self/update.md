# `update`

Performs an update of rye.

This can install updates from the latest release binaries or trigger a manual
compilation of Rye if Rust is installed.

## Example

Update to the latest version:

```
$ rye self update
```

Update (or downgrade)  to a specific version:

```
$ rye self update --version 0.20
```

Compile a specific revision:

```
$ rye self update --rev 08910bc9b3b7c72a3d3ac694c4f3412259161477
```

Compile latest development version:

```
$ rye self update --branch main
```

## Arguments

_no arguments_
    
## Options

* `--version <VERSION>`: Update to a specific version

* `--tag <TAG>`: Update to a specific tag

* `--rev <REV>`: Update to a specific git rev

* `--branch <BRANCH>`: Update to a specific git branch

* `--force`: Force reinstallation

* `-h, --help`: Print help (see a summary with '-h')
