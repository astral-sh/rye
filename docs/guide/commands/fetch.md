# `fetch`

Fetches a Python interpreter for the local machine.  This command is
available under the aliases `rye fetch` and `rye toolchain fetch`.

As of Rye 0.31.0 toolchains are always fetched without build info.  This
means that in the folder where toolchains are stored only the interpreter
is found.  For more information see [Fetching Toolchains](../toolchains/index.md#build-info).

## Example

Fetch a specific version of Python:

```
$ rye fetch 3.8.13
Downloading cpython@3.8.13
Checking checksum
Unpacking
Downloaded cpython@3.8.13
```

To fetch the pinned version of Python you can leave out the argument:

```
$ rye fetch
Downloading cpython@3.8.17
Checking checksum
Unpacking
Downloaded cpython@3.8.17
```

To fetch a version of Python into a specific location rather than rye's
interpreter cache:

```
$ rye fetch cpython@3.9.1 --target-path=my-interpreter
```

## Arguments

* `[VERSION]`: The version of Python to fetch.

    If no version is provided, the requested version will be fetched.

## Options

* `-f, --force`: Fetch the Python toolchain even if it is already installed.

* `--target-path` `<TARGET_PATH>`: Fetches the Python toolchain into an explicit location rather

* `--build-info`: Fetches with build info

* `--no-build-info`: Fetches without build info

* `-v, --verbose`: Enables verbose diagnostics

* `-q, --quiet`: Turns off all output

* `-h, --help`: Print help (see a summary with '-h')