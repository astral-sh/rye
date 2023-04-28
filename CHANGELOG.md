# Rye Changes

There is currently no actual release of Rye.  The latest main branch revision
is the one you can install as mainline should always be stable.  Here are the
most recent changes however.

## April

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
