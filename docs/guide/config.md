# Configuration

Most of Rye's configuration is contained within the `pyproject.toml` file.  There is however
also a bit of global configuration to influence how it works.

## Changing Home Folder

By default Rye places all it's configuration in `~/.rye` on Unix and `%USERPROFILE%\.rye` on
Windows.  This behavior can be changed via the `RYE_HOME` environment variable.  This is useful
if you do not like the default location of where Rye places it's configuration or if you need
to isolate it.

## Home Folder Structure

The `.rye` home folder contains both user configuration as well as Rye managed state such
as [installed toolchains](toolchains/index.md).  The following files and folders are placed within the
`.rye` folder.  Note that not all are there always.

### `config.toml`

This is a configuration file that influences how Rye operates.  Today very little configuration
is available there.  For the available config keys see [Config File](#config-file).

### `self`

While Rye is written in Rust, it uses a lot of Python tools internally.  These are maintained in
an internal virtualenv stored in this location.

### `py`

In this folder Rye stores the different [toolchains](toolchains/index.md).  Normally those are folders
containing downloaded Python distributions, but they can also be symlinks or special reference
files.

### `shims`

This folder contains shim binaries.  These binaries are for instance the `python` executable
which automatically proxies to the current virtualenv or globally installed [tools](tools.md).

## Config File

The config file `config.toml` in the `.rye` folder today only is used to manage defaults.  This
is a fully annotated config file:

```toml
[default]
# This is the default value that is written into new pyproject.toml
# files for the `project.requires-python` key
requires-python = ">= 3.8"

# This is the default toolchain that is used
toolchain = "cpython@3.11.1"

# This is the default build system that is used
build-system = "hatchling"

# This is the default license that is used
license = "MIT"

# This sets the default author (overrides the defaults from git).  The
# format here is "Name <email>".
author = "Full Name <email@address.invalid>"

# The dependency operator to use by default for dependencies.  The options are
# '>=', '~=', and '=='.  The default currently is '>='.  This affects the behavior
# of `rye add`.
dependency-operator = ">="

[proxy]
# the proxy to use for HTTP (overridden by the http_proxy environment variable)
http = "http://127.0.0.1:4000"
# the proxy to use for HTTPS (overridden by the https_proxy environment variable)
https = "http://127.0.0.1:4000"

[behavior]
# When set to true the `managed` flag is always assumed to be true.
force-rye-managed = false

# Enables global shims when set to `true`.  This means that the installed
# `python` shim will resolve to a Rye managed toolchain even outside of
# virtual environments.
global-python = false

# When set to `true` enables experimental support of uv as a replacement
# for pip-tools. Learn more about uv here: https://github.com/astral-sh/uv
use-uv = false

# Marks the managed .venv in a way that cloud based synchronization systems
# like Dropbox and iCloud Files will not upload it.  This defaults to true
# as a .venv in cloud storage typically does not make sense.  Set this to
# `false` to disable this behavior.
venv-mark-sync-ignore = true

# a array of tables with optional sources.  Same format as in pyproject.toml
[[sources]]
name = "default"
url = "http://pypi.org/simple/"
```

## Manipulating Config

+++ 0.9.0

The configuration can be read and modified with `rye config`.  The
keys are in dotted notation.  `--get` reads a key, `--set`, `--set-int`,
`--set-bool`, or `--unset` modify one.

```bash
rye config --set proxy.http=http://127.0.0.1:4000
rye config --set-bool behavior.force-rye-managed=true
rye config --get default.requires-python
```

For more information see [`config`](commands/config.md).

## Per Project Config

For the project specific `pyproject.toml` config see [pyproject.toml](pyproject.md).
