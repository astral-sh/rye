# `add`

Adds a Python package to this project.  The command takes a PEP 508 requirement string
but provides additional helper arguments to make this process more user friendly.  For
instance instead of passing git references within the requiement string, the `--git`
parameter can be used.

After a dependency is added it's not automatically installed.  To do that, you need to
invoke the [`sync`](sync.md) command.  To remove a dependency again use the [`remove`](remove.md)
command.

## Example

Add the latest version of a dependency that is compatible with the configured Python version:

```
$ rye add flask
Added flask>=3.0.1 as regular dependency
```

Add a dependency but add an optional extra feature:

```
$ rye add flask --features dotenv
Added flask[dotenv]>=3.0.1 as regular dependency
```

Add a git dependency:

```
$ rye add flask --git https://github.com/pallets/flask
Added flask @ git+https://github.com/pallets/flask as regular dependency
```

## Arguments

* `<REQUIREMENTS>...`: The package to add as PEP 508 requirement string. e.g. 'flask==2.2.3'

## Options

* `--git <GIT>`: Install the given package from this git repository

* `--url <URL>`: Install the given package from this URL

* `--path <PATH>`: Install the given package from this local path

* `--absolute`: Force non interpolated absolute paths

* `--tag <TAG>`: Install a specific tag

* `--rev <REV>`: Update to a specific git rev

* `--branch <BRANCH>`: Update to a specific git branch

* `--features <FEATURES>`: Adds a dependency with a specific feature

* `--dev`: Add this as dev dependency

* `--excluded`: Add this as an excluded dependency that will not be installed even if it's a sub dependency

* `--optional <OPTIONAL>`: Add this to an optional dependency group

* `--pre`: Include pre-releases when finding a package version

* `--pin <PIN>`: Overrides the pin operator [possible values: `equal`, `tilde-equal``, `greater-than-equal``]

* `-v, --verbose`: Enables verbose diagnostics

* `-q, --quiet`: Turns off all output

* `-h, --help`: Print help (see a summary with '-h')