# `publish`

Publish packages to a package repository.  This publishes the packages which are
produced by the build command.

For more information see [Building and Publishing](../publish.md).

## Example

Build and publish:

```
$ rye build
$ rye publish
```

Publish a specific artifact:

```
$ rye publish dist/example-0.1.0.tar.gz
```

## Arguments

* `[DIST]...`: The distribution files to upload to the repository (defaults to `<workspace-root>/dist/*`)

## Options

* `-r, --repository <REPOSITORY>`: The repository to publish to [default: `pypi`]

* `--repository-url <REPOSITORY_URL>`: The repository url to publish to

* `-u, --username <USERNAME>`: The username to authenticate to the repository with

* `--token <TOKEN>`: An access token used for the upload

* `--sign`: Sign files to upload using GPG

* `-i, --identity <IDENTITY>`: GPG identity used to sign files

* `--cert <CERT>`: Path to alternate CA bundle

* `-y, --yes`: Skip prompts

* `-v, --verbose`: Enables verbose diagnostics

* `-q, --quiet`: Turns off all output

* `-h, --help`: Print help (see a summary with '-h')