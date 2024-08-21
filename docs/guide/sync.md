# Syncing and Locking

Rye uses [`uv`](https://github.com/astral-sh/uv) to manage dependencies.

In order to download dependencies rye creates two "lockfiles" (called
`requirements.lock` and `requirements-dev.lock`).  These are not real lockfiles,
but they fulfill a similar purpose until a better solution has been implemented.

Whenever `rye sync` is called, it will update lockfiles as well as the
virtualenv.  If you only want to update the lockfiles, then `rye lock` can be
used.

## Lock

When locking, some options can be provided to change the locking behavior.  These flags are
also all available on `rye sync`.

### `--update` / `--update-all`

Updates a specific or all requirements to the latest and greatest version.  Without this flag
a dependency will only be updated if necessary.

```
rye lock --update-all
```

### `--features` / `--all-features`

Python packages can have extra dependencies.  By default the local package that is installed
will only be installed with the default features.  If for instance you have an extra dependency
this will only be installed if the feature is enabled.

```
rye add --optional=web flask
rye lock --features=web
```

When working with [workspaces](../workspaces/), the package name needs to be prefixed with a slash:

```
rye lock --features=package-name/feature-name
```

The `--features` parameter can be passed multiple times and features can also be comma
separated.  To turn on all features, the `--all-features` parameter can be used.

```
rye lock --all-features
```

### `--pre`

By default updates and version resolution will not consider pre-releases of packages.  If you
do want to include those, pass `--pre`

```
rye lock Flask --pre
```

### `--with-sources`

+++ 0.18.0

By default (unless the `tool.rye.lock-with-sources` config key is set to `true` in the
`pyproject.toml`) lockfiles are not generated with source references.  This means that
if custom sources are used the lockfile cannot be installed via `uv` or `pip`, unless
`--find-links` and other parameters are manually passed.  This can be particularly useful
when the lockfile is used for Docker image builds.

When this flag is passed then the lockfile is generated with references to `--index-url`,
`--extra-index-url` or `--find-links`.

```
rye lock --with-sources
```

## Sync

Syncing takes the same parameters as `lock` and then some.  Sync will usually first do what
`lock` does and then use the lockfiles to update the virtualenv.

### `--no-lock`

To prevent the lock step from automatically running, pass `--no-lock`.

```
rye sync --no-lock
```

### `--no-dev`

Only sync based on the production lockfile (`requirements.lock`) instead of the development
lockfile (`requirements-dev.lock`).

```
rye sync --no-dev
```

## Platform Compatibility

By default, lockfiles depend on the platform they were generated on.

For example, if your project relies on platform-specific packages and you generate
lockfiles on Windows, these lockfiles will include Windows-specific projects.
Consequently, they won't be compatible with other platforms like Linux or macOS.

To generate a cross-platform lockfile, you can enable uv's `universal` setting
by adding the following to your `pyproject.toml`:

```toml
[tool.rye]
universal = true
```
