# Rye Changes

There is currently no actual release of Rye.  The latest main branch revision
is the one you can install as mainline should always be stable.  Here are the
most recent changes however.

## April

- Rye now supports negative (exclusion) dependencies.  These can be used to
  prevent a dependency from installing, even if something else in the graph
  depends on it.  Use `rye add --exclude package-name` to add such a dependency.

- `sync` now accepts `--no-lock` to prevent updating the lock file.

- Rye's `add` command now accepts a `--pre` parameter to include pre-release.

- Rye's `pin` command now updates the pyproject.toml requires-python.

- Rye's `install` command now accepts a `--include-dep` parameter to include
  scripts from one or more given dependencies.

- Rye now honors `requires-python` in the `add` command.  This means the the
  initial resolution will not pick a version higher than what's supported by
  the lower boundary.

- When installing packages as global tools, a warning is now emitted if there
  were no scripts in the package.  Additionally installing packages from local
  paths and zip files is now supported.

- A `rye self update` command was added to compile and install the latest
  version via cargo.

- Added more convenient ways to install from git/urls by supplying a `--git`
  or `--url` parameter.  This will behind the scenes format a PEP 508 requirement
  string.

- Added a `shell` command which will spawn a shell with the virtualenv activated.

- Added a `make-req` command to conveniently format out PEP 508 requirement
  strings from parts.

- The internal virtualenv used to manage `pip-tools` and other libraries now
  automatically updates when necessary.

- `rye toolchain register` can now be used to register a local python installation
  as toolchain with rye.

- `rye build` was added to allow building `sdist` and `bdist_wheel` distributions.

- Rye now correctly handles whitespace in folder names.
