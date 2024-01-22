# `make-req`

Builds and prints a PEP 508 requirement string from parts.  This is a utility command
that rarely needs to be used but can help creating requirements strings for pasting into
other tools.  It takes the same arguments as [`add`](add.md) but rather than adding the
requirements into the requirements file it just spits out a formatted PEP 508 requirement
string on stdout.

## Example

```
$ rye make-req flask --git https://github.com/pallets/flask --rev 4df377cfbf
flask @ git+https://github.com/pallets/flask@4df377cfbf
```

## Arguments

* `[REQUIREMENTS]...` The package to add as PEP 508 requirement string. e.g. `'flask==2.2.3'`

## Options

* `--git <GIT>`: Install the given package from this git repository

* `--url <URL>`: Install the given package from this URL

* `--path <PATH>`: Install the given package from this local path

* `--absolute`: Force non interpolated absolute paths

* `--tag <TAG>`: Install a specific tag

* `--rev <REV>`: Update to a specific git rev

* `--branch <BRANCH>`: Update to a specific git branch

* `--features <FEATURES>`: Adds a dependency with a specific feature

* `-h, --help`: Print help (see a summary with '-h')