# Changelog

This file contains tracks the changes landing in Rye.  It includes changes
that were not yet released.

<!-- released start -->

## 0.35.0

Released on 2024-06-24.

* Enforce `--pre` when auto-syncing by @charliermarsh in https://github.com/astral-sh/rye/pull/1107
* Move from `rye-up.com` to `rye.astral.sh` by @charliermarsh in https://github.com/astral-sh/rye/pull/1113
* Fix install instructions on README.md for mac/linux by @timothycrosley in https://github.com/astral-sh/rye/pull/1114
* Fix CLI deserialization of PowerShell (`powershell`) by @charliermarsh in https://github.com/astral-sh/rye/pull/1125
* Add ability to specify option to generate hashes within pyproject.toml by @asmith26 in https://github.com/astral-sh/rye/pull/1129
* Bump uv to 0.2.13 by @charliermarsh in https://github.com/astral-sh/rye/pull/1123
* Improve `config.toml` error messages by @zys864 in https://github.com/astral-sh/rye/pull/1155

## 0.34.0

Released on 2024-05-20.

* Add nushell completion support by @MilesCranmer in https://github.com/astral-sh/rye/pull/1030
* Use uv in rye build when enabled by @bluss in https://github.com/astral-sh/rye/pull/978
* Add short version add -d for rye add --dev by @bluss in https://github.com/astral-sh/rye/pull/1044
* Flip uv to the default Rye backend by @charliermarsh in https://github.com/astral-sh/rye/pull/1053
* Fix Rye not using user-chosen toolchain as default during installation by @pjdon in https://github.com/astral-sh/rye/pull/1054
* Add keyring support for uv by @emarsden-iso in https://github.com/astral-sh/rye/pull/1016
* Allow to generate lockfiles with hashes when using uv by @mvaled in https://github.com/astral-sh/rye/pull/1070
* Bump ruff to 0.4.4 by @davfsa in https://github.com/astral-sh/rye/pull/1075
* Fix TOML array formatting by @my1e5 in https://github.com/astral-sh/rye/pull/1084
* Bump uv to 0.1.44 by @charliermarsh in https://github.com/astral-sh/rye/pull/1085
* Discover cosmo-ified (`.com`) binaries on Windows by @mataha in https://github.com/astral-sh/rye/pull/1091
* Write `use-uv = true` in no-prompt mode by @charliermarsh in https://github.com/astral-sh/rye/pull/1098

## 0.33.0

Released on 2024-04-24.

- Ensure files created by `rye init`, such as `pyproject.toml` and initial python files end with a newline. #979

- Add `--refresh` argument on `-f`.  #994

- Preserve trailing newline in templates.  #979

- Update uv to 0.1.37.  #980

- Allow comments in `.python-version`.  #1038

- Update Python releases to include 3.12.3 et al.  #1022

## 0.32.0

Released on 2024-03-29

- Update uv to 0.1.26.  #924

- Always create `.gitignore` file in `rye init`.  #919

- Prevent `rye fetch --force` from removing a target directory that is not a Python installation.  #921

- `rye list` always prints the currently installed packages even this project is not managed by Rye.  #940

- Fix error on using -v or -q with `rye fmt` or `rye lint`. #959

- Fix rye fetch detection of registered toolchain.  #931

- Ignore build-system configuration for virtual projects.  #929

## 0.31.0

Released on 2024-03-22

- Update uv to 0.1.23.  #916

- Allow `rye publish` working outside of project.  #910

- `rye test --quiet` no longer implies `--no-capture`. #915

- Rye now can be used to fetch Python installations even when not using Rye
  and build infos are no longer included by default.  This means that rather
  than having interpreters at `~/.rye/py/cpython@3.11.1/install/bin/python3`
  it will now reside at `~/.rye/py/cpython@3.11.1/bin/python3`.  #917

- Installer now recommends `uv` over `pip-tools`.  #918

## 0.30.0

Released on 2024-03-19

- Update uv to 0.1.21.  #884, #890, #904

- Fix incorrect flag passing of `rye test` `-q` and `-v`.  #880

- Rye now loads `.env` files.  This applies both for Rye's own
  use of environment variables but also to scripts launched via
  `run`.  #894

- Fix `rye add m --path ./m` causing a panic on windows.  #897

## 0.29.0

Released on 2024-03-11

- Updated to `uv` 0.1.17.  #850, #867

- Trap panics and silence bad pipe errors.  #862

- Updating `rye` will now also ensure that the self-venv is updated.  Previously
  this was deferred until the next `sync`.  #863

- The `self update` command now accepts `--branch`.  #864

- Fixed an issue that caused pip-tools to not update.  #865

- Updates `build` and `certifi`.  #866

## 0.28.0

Released on 2024-03-07

- `--skip-existing` is now available with Rye's `publish` command. #831

- Bumped `uv` to 0.1.15.  #760, #820, #837

- Bumped `ruff` to 0.3.0.  #821

- The `init` command now generates a script with the name of the
  project rather than `hello`.  #801

- Retain markers when adding dependencies with features when uv is used.  #807

- Fixed a bug that caused repeated syncs not to recall all previous options.  #830

- Report `self-python` version in `--version`.  #843

- Fixes a bug where `rye config` would not create the `RYE_HOME` folder if needed.  #844

- `rye add` now retains version and URL for the requirements when `uv` is used.  #846

- Added a `rye test` command which invokes `pytest`.  #847

## 0.27.0

Released on 2024-02-26

- rye now uses `uv` to bootstrap its internal packages and tools. #754

- rye no longer fails if an incorrect `VIRTUAL_ENV` environment
  variable is exported.  #766

- Added latest Python builds.  #771

- When `uv` is used the prompt is now set to the project name.  #773

- Allow `rye fetch --force` to force re-fetch a downloaded toolchain.  #778

- Fixed a panic when adding a package to a virtual project.  #783

- Bumped `uv` to 0.1.11.  #790

## 0.26.0

Released on 2024-02-23

- `init` now supports `--script` and `--lib` to generate a script or library project.  #738

- Fixed `rye config --show-path` abort with an error. #706

- Bumped `uv` to 0.1.9.  #719, #740, #746

- Bumped `ruff` to 0.2.2.  #700

- Prevent `rye toolchain remove` from removing the currently active toolchain.  #693

- Sync latest PyPy releases. #683

- Fixes an issue where when `uv` is enabled, `add` did not honor custom sources.  #720

- When `uv` is enabled, rye will now automatically sync on `add` and `remove`.  #677

- Rename `rye tools list` flags: `-i, --include-scripts` to `-s, --include-scripts` and `-v, --version-show` to `-v, --include-version`.  #722

## 0.25.0

Released on 2024-02-19

- Improved the error message if `config` is invoked without arguments.  #660

- Bump `uv` to 0.1.5.  #665, #675, #698

- When `uv` is enabled, `rye add` now uses `uv` instead of `unearth`
  internally.  #667

- The installer now has slightly better wording for what the shims are doing.  #669

- `uv` can now also be enabled on windows.  #675

- Removed the unsupported and un-used `arch` parameter from `fetch`.  #681

- Fixed the `-q` parameter not working for the `init` command.  #686

- `rye tools list` shows broken tools if the toolchain was removed. #692

- Configure the ruff cache directory to be located within the workspace root. #689

- Use default toolchain to install tools.  #666

- `rye --version` now shows if `uv` is enabled.  #699

## 0.24.0

Released on 2024-02-15

- Added new `rye list` command and deprecated `rye show --installed-deps` which it replaces.  #656

- Added experimental support for `uv`.  #657

## 0.23.0

Released on 2024-02-13

- When `behavior.venv-mark-sync-ignore` is set to `false` and the file system
  does not support extended attributes, no longer will a warning be printed.  #633

- Fixed a bug that caused warnings about unsupported operations to be shown on Linux. #634

- The venv sync marker is now only updated when a new virtualenv is created.  #638

- Lockfiles now contain annotations.  #643

## 0.22.0

Released on 2024-02-09

- Virtual envs managed by Rye will now by default be marked to not sync to
  known cloud storage systems (Dropbox and iCloud).  #589

- Fixed a bug where pip-tools sometimes did not get initialized.  #596

- Rye now prefers installed toolchains over newer latest toolchains unless
  a precise pin is used.  #598

- Removed the non functional `shell` command.  #602

- Upgraded internal unearth dependency which resolved an issue where
  `rye add tensorflow` would not work.  #614

- The installer now supports `RYE_TOOLCHAIN_VERSION`.  #606

- `rye init` will no longer create packages with leading digits.  #616

- Rye now statically links `vcruntime` on Windows which no longer requires
  the vs redist to be installed.  #622

- `rye show` now prints out which sources are configured for a project.  #631

## 0.21.0

Released on 2024-02-03

- `rye fetch` now is able to fetch impliciit version in all cases.  Previously
  global shims were not properly defaulted which required the user to be explicit
  with the fetch request.  #574

- The rye installer now prompts for the default toolchain version if global shims
  are enabled.  #576

- The internal Python version was bumped to 3.12.  #576

- The installer now can automatically add Rye to `PATH` on most UNIX environments.  #580

## 0.20.0

Released on 2024-02-01

- Improved the error message when an update could not be performed because files
  are in use.  #550

- Rye now supports virtual projects.  These are themselves not installed into the
  virtualenv but their dependencies are.  #551

- Update the Python internals (python external dependencies) to new versions.  #553

- Update to newer versions of pip tools.  For Python 3.7 `6.14.0` is used, for
  new Python versions `7.3.0` is used.  #554

- Added `rye fmt` and `rye lint` commands to format and lint with
  the help of Ruff.  #555

- Restore cursor state on Ctrl-C.  This fixes some issues where in rare cases the
  cursor would disappear even after shutting down rye.  #564

- Upon installation Rye now prompts if global shims should be enabled.  #566

- Add a warning about bugs to the `shell` command until the behavior has been
  fixed.  #567

## 0.19.0

Released on 2024-01-21

- Improved the behavior of `rye fetch`.  When invoked without arguments it will now try to
  fetch the version of the requested Python interpreter.  Specifically this combining
  `pin` and `fetch` work in a much simplified manner.  #545

- Fixed an issue where `rye init` would pin a much too specific version in the `.python-version`
  file that is generated.  #545

- On Windows the `PATH` is now automatically adjusted on install and uninstall.  This means that
  manually adding the rye folder to the search path is no longer necessary.  #483

- Fixed a regression in 0.18 that caused the `add` command to fail.  #547

## 0.18.0

Released on 2024-01-20

- Incorporate new Python builds.  #535

- Disable revocation checks on windows to support corporate MITM proxies.  #537

- Detect when a virtualenv relocates and automatically re-create it on sync.  #538

- Added `lock --with-sources`, `sync --with-sources` and the new `rye.tool.lock-with-sources`
  config.  Passing this will ensure that source references are included in the
  lock files.  #540

- When using global python shims, the `.python-version` file is now correctly
  picked up in all cases.  #541

- Added a helpful message if someone attempts to run the non existing `rye list`
  command.  At a later point there should be a real listing command that can print
  out the dependencies.  Today the only option is the `--installed-deps` option on
  the `show` command which spits out dependencies in the format of the lockfile.  #543

- The installer will no longer attempt to symlink targets which are not valid
  executables on the platform.  This works around some issues with Packages that
  would prevent to install such as `changedetection.io`.  #542

## 0.17.0

Released on 2024-01-15

- Fixed default generated script reference.  #527

- Correctly fall back to home folder if HOME is unset.  #533

## 0.16.0

Released on 2023-12-17

- By default a script with the name of the project is now also configured.  #519

- Rye now configures hatchling better in `rye init` so that it works with
  hatchling 1.19 and later.  #521

- Rye now detects the dummy Python shim that starts the windows store and
  refuses to consider it.  #486

## 0.15.2

Released on 2023-10-04

- Fixed the updater not replacing the python shim correctly on Linux.

## 0.15.1

Released on 2023-10-03

- Fixed the updater not replacing the python3 shim correctly.

## 0.15.0

Released on 2023-10-03

- Added support for Python 3.12.  #462

## 0.14.0

Released on 2023-10-01

- Add support for fetching alternative CPU architectures.  #447

- The order of git submodule initialization was changed.  This improves the
  automatic author detection when `includeIf` is used.  #443

- The linux shim installer code will no longer fall back to symlinks when a
  hardlink cannot be created.  This is done as a symlinked shim will not
  ever function correctly on Linux.  This prevents the shim executables like
  `python` to instead act as if they are `rye`.  The fallback behavior is now
  to copy the executable instead.  #441

- The installer now detects `fish` and will spit out additional instructions
  for configuring the shell.

- Fix the wrong behavior when bump version.  #454

## 0.13.0

Released on 2023-08-29

- Add a `python3` shim on windows.  Previously entering `python3` in the
  command line would always bring up the windows store python proxy even
  when global shims were enabled.  As virtualenvs do not support the
  `python3` executable on windows, the internal shim handling is now also
  changed so that trying to launch `python3` will fall back to `python`.
  This makes it possible to run `maturin build`.

- Add `maturin` build command to start a new maturin PyO3 project.

## 0.12.0

Released on 2023-08-27

- Improve handling of the pth files for TCL on pypy. #409

- The `rye tools list` command now accepts `-v` to also print out the
  versions of the installed tools. #396

- Fixed parsing of versions by `rye version`. #397

- Improved the help message for `rye init`. #401

- The email address now defaults to a syntactically valid email address
  if not known to prevent errors with some build tools.

- Added new Python versions.

- The rye installer now detects `NOEXEC` temporary folders and prints out
  a more helpful error message. #394

- Fixed an issue where the author email was incorrectly detected. #382

- The prompt of new virtualenvs is now set to the project name. #383

## 0.11.0

Released on 2023-07-18

- Added new Python versions.

- Added a new config key `default.author` to configure the default author
  that should be set.  This overrides the default author that is normally
  loaded from the git config.  #377

- When importing with `rye init` and no `src` folder exists, it will not be
  created.  #375

- Added support for `shell` command on Windows.  #363

- Pin down pip to an older version to avoid issues with an incompatible
  `pip-tools` version.  This does not yet update pip-tools to 7.0 as there
  are significant regressions in 7.x. #374

- The `version` command can show dynamic versions now. #355

- `rye add` now properly checks some incompatible argument combinations.  #347

- There is now more toolchain validation.  This better supports cases where
  rye was interrupted during sync.  #351

## 0.10.0

Released on 2023-07-07

- Fixed a bug with `rye init` not operating correctly due to a argument conflict.  #346

- Scripts now support a PDM style `call` script type.  #345

- The `init` command is now capable of importing existing projects.  #265

- Fixed the global shim behavior on Windows.  #344

## 0.9.0

Released on 2023-06-21

- The internal Rye Python version is now 3.11.

- Rye now emits most messages, most of the time to stdout rather than stderr.  #342

- `rye add` now accepts `--pin` to let one override the type of pin to use.  #341

- Added `rye config` to read and manipulate the `config.toml` file.  #339

- Added support for the new `behavior.global-python` flag which turns on global
  Python shimming.  When enabled then the `python` shim works even outside of
  Rye managed projects.  Additionally the shim (when run outside of Rye managed
  projects) supports a special first parameter `+VERSION` which requests a
  specific version of Python (eg: `python +3.8` to request Python 3.8).  #336

- Renamed the config key `default.dependency_operator` to `default.dependency-operator`
  and `behavior.force_rye_managed` to `behavior.force-rye-managed`.  #338

## 0.8.0

Released on 2023-06-18

- Rye for now prefers `>=` over `~=` for newly added dependencies.

- The workspace member declaration is now platform independent.  If `members` is
  now explicitly set to an empty list it will not fall back to auto discovery.  #331

- `rye add` now pins versions with `==` instead of `~=` when the version of the
  package does not use at least two components.  This means that for instance it
  will now correctly use `openai-whisper==20230314` rather than
  `openai-whisper~=20230314` which is not actually satisfiable.  #328

- `rye install` now lets you install dependencies into the tool's virtualenv
  during installation that are undeclared via the new `--extra-requirement`
  option.  #326

- Improved handling of relative path installations by setting `PROJECT_ROOT`
  the same way as PDM does.  #321

- Workspaces will now never discover `pyproject.toml` files in any dot
  directories. (Name starting with `.`)  #329

- Fixed `rye build` not working correctly on Windows.  #327

## 0.7.0

Released on 2023-06-12

- `rye sync` and `rye lock` now accept `--pyproject`.  #296

- Added JSON output to `rye toolchain list` by adding `--format=json`.  #306

- `rye version` can bump version by `--bump` option now.  #298

- Fixed members not handled correctly in workspaces.  #300

- Add `--clean` for `build` command.  #297

- Fixed an issue where pip was not invoked from the right working directory
  causing issues for workspace installations.  #292

- `rye init` now accepts `--private` to set the `Private :: Do Not Upload` classifier
  that prevents uploads to PyPI.  #291

## 0.6.0

Released on 2023-06-03

- Add `version` subcommand for rye. #285

- Fixed `rye pin` pinning the wrong version.  #288

- Calling `rye init` on the root directory no longer fails.  #274

- `rye run`, `show`, `pin`, `shell` and `build` now take a `--pyproject`
  argument. #232

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
