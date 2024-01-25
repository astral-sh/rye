# `fetch`

Fetches a Python interpreter for the local machine.  This command is
available under the aliases `rye fetch` and `rye toolchain fetch`.

## Example

Fetch a specific version of Python:

```
$ rye fetch 3.8.13
Downloading cpython@3.8.13
Checking checksum
success: Downloaded cpython@3.8.13
```

To fetch the pinned verison of Python you can leave out the argument:

```
$ rye fetch
Downloading cpython@3.8.17
Checking checksum
success: Downloaded cpython@3.8.17
```

## Arguments

* `[VERSION]`: The version of Python to fetch.

    If no version is provided, the requested version will be fetched.

* `[ARCH]`: Overrides the architecture to fetch.

    When a non native architecture is fetched, the toolchain is installed under an alias.
    
## Options

* `-v, --verbose`: Enables verbose diagnostics

* `-q, --quiet`: Turns off all output

* `-h, --help`: Print help (see a summary with '-h')