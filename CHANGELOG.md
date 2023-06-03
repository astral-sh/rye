# Changelog

This file contains tracks the changes landing in Rye.  It includes changes
that were not yet released.

## 0.7.0

_Unreleased_

<!-- released start -->

## 0.6.0

Released on 2023-06-03

- Add `version` subcommand for rye. #285

- Fixed `rye pin` pinning the wrong version.  #288

- Calling `rye init` on the root directory no longer fails.  #274

## 0.5.0

Released on 2023-05-31

- Rye will no longer enforce a downloaded interpreter for the internal
  toolchain.  If one has been registered that is compatible it will be
  used.  Additionally the installer now supports the `RYE_TOOLCHAIN`
  environment variable which allows a user to supply an already existing
  Python interpreter at install time.  #267

- The `publish` command now supports `--yes` to disable prompts.  #270

- When a Python debug build (`Py_DEBUG`) is registered as custom toolchain,
  `-dbg` is automatically appended to the name by default.  #269

- lto+pgo builds are now preferred for the Python toolchain builds when
  available.  #268

- It's now possible for `.python-version` to request partial Python versions
  in which case the latest available is used.  In particular this means that
  a version like `3.10` can be written into `.python-version` rather than
  `3.10.11`.  This can be accomplished by invoking `pin` with the new
  `--relaxed` flag.  #255

- Workspaces will no longer discover `pyproject.toml` files in virtualenvs
  or `.git` folders.  #266

- Adding or removing dependencies with `add` or `remove` now reformats
  the `dependencies` array in the `pyproject.toml` file to multi-line
  with trailing commas.  This should result in significantly better
  diffing behavior out of the box.  #263

- Default build-system and license can be specified in global config.  #244

- Fixed an issue where the `init` command would not let you create
  `flit` based projects.  #254

- Resolve an error ("No such file or directory") shown after updates on
  Linux machines.  #252

- The built-in updater now validates checksums of updates when updates have
  SHA-256 hashes available.  #253

- `init` now accepts `--no-pin` to not create a `.python-version` file.  #247

## 0.4.0

Released on 2023-05-29

- Releases starting with `0.4.0` onwards are published with SHA256 checksum
  files for all release assets.  These files are not yet validated by the
  installer or updater however.

- The `install` command can now install tools from custom indexes.  #240

- Virtualenvs on Unix are now created with a hack to pre-configure TCL and
  TKinter.  #233

- Fix invalid version error when using rye init with custom toolchain.  #234

- Failed tool installations now properly clean up.  #225

- Correctly swap the rye executable on windows when performing an update
  to a git version via `self update`.

## 0.3.0

Released on 2023-05-27

- Support retrieving username and repository-url from credentials if not
  provided for the `publish` command.  #217

- The installer now validates the availability of shared libraries
  on Linux with `ldd` and emits an error with additional information
  if necessary shared libraries are missing.  #220

- It's now possible to configure http and https proxies.  #215

- If a package is not found because it only has matching pre-releases,
  a warning is now printed to tell the user to pass `--pre`.  #218

- Add `--username` parameter for rye publish.  #211

- The shims are now more resilient.  Previously a `pyproject.toml` file
  caused in all cases a virtualenv to be created.  Now this will only
  happen when the `rye.tool.managed` flag is set to `true`.  The old
  behavior can be forced via the global config.  #212

## 0.2.0

Released on 2023-05-23

- Resolved a bug where on Windows hitting the shift key (or some other keys)
  in confirm prompts would cause an error.

- The installer on Windows now warns if symlinks are not enabled and directs
  the user to enable developer mode.  The `--version` output now also
  shows if symlinks are available.  #205

- Support auto fix requires-python when there is a conflict. #160

- Added support for custom indexes.  #199

- `rye add` no longer complains when a local version information is
  in the version.  #199

## 0.1.2

Released on 2023-05-22

- Fixed dev-dependencies not being installed when using workspace.  #170

- `init` no longer creates invalid flit config.  #195

- Support direct references when adding a package.  #158

- Fixed a bug with uninstall on Unix platforms.  #197

## 0.1.1

Released on 2023-05-18

- The installer on windows will now ask for a key to be pressed so it does
  not close the window without information.  #183

- Fixed an issue on macOS where the installer would die with "os error 24"
  when directly piped to bash.  #184

## 0.1.0

Released on 2023-05-17

- Rye now comes with binary releases for some platforms.

- A new `self uninstall` command was added to uninstall rye and the new
  `self update` command updates to the latest release version.

- Rye now includes a `publish` command for publishing Python packages to a
  package repository.  #86

- Script declarations in `pyproject.toml` now permit chaining and custom
  environment variables.  #153

- Added `tools install` and `tools uninstall` as aliases for `install` and
  `uninstall` and added `tools list` to show all installed tools.

- Rye is now capable of downloading a selected set of PyPy releases.  To do
  so use `rye pin pypy@3.9.16` or any other supported PyPy release.

- Custom cpython toolchains are now registered just as `cpython` rather
  than `custom-cpython`.

- Rye now supports Python down to 3.7.

- Rye's `self` command now includes a `completion` subcommand to generate
  a completion script for your shell.

- The downloaded Python distributions are now validated against the
  SHA-256 hashes.

- Rye now builds on windows.  This is even more experimental though
  than support for Linux and macOS.

- Added `--features` and `--all-features` for `lock` and `sync`.

- Rye will now look at the `RYE_HOME` to determine the location of the
  `.rye` folder.  If it's not set, `$HOME/.rye` is used as before.

- Rye now has a most consistent handling for virtualenv versions.  If
  `.python-version` is provided, that version is used.  Otherwise if
  `requires-python` is set in the `pyproject.toml`, that version is used
  instead.  When a new project is created the `.python-version` file is
  written and the current latest cpython version is picked.

- It's now possible to explicitly set the `name` of the project when
  initializing a new one.

- Rye's `init` command now attempts to initialize projects with `git` and
  will automatically create a `src/project_name/__init__.py` file.

- Rye can now also generate a license text when initializing projects.

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
