# `version`

Get or set project version.  Note that this does not refer to the version of Rye
itself but the version that is set in the `pyproject.toml` file.

## Example

Get the current version:

```
$ rye version
0.1.0
```

Bump the version by minor:

```
$ rye version -b minor
version bumped to 0.2.0
```

Set to a specific version:

```
$ rye version 1.0.0
version set to 1.0.0
```

## Arguments

* `[VERSION]`: the version to set

## Options

* `-b, --bump <BUMP>`: automatically bump the version in a specific way (`major`, `minor` or `patch`)

* `-h, --help`: Print help (see a summary with '-h')