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

[proxy]
# the proxy to use for HTTP (overridden by the http_proxy environment variable)
http = "http://127.0.0.1:4000"
# the proxy to use for HTTPS (overridden by the https_proxy environment variable)
https = "http://127.0.0.1:4000"

[behavior]
# When set to true the `managed` flag is always assumed to be true.
force_rye_managed = false

# a array of tables with optional sources.  Same format as in pyproject.toml
[[sources]]
name = "default"
url = "http://pypi.org/simple/"
```

## Per Project Config

For the project specific `pyproject.toml` config see [pyproject.toml](pyproject.md).
