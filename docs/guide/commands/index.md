# Commands

This is a list of all the commands that rye provides:

* [add](add.md): Adds a Python package to this project
* [build](build.md): Builds a package for distribution
* [config](config.md): Reads or updates the Rye configuration
* [fetch](fetch.md): Fetches a Python interpreter for the local machine (alias)
* [fmt](fmt.md): Run the code formatter on the project
* [init](init.md): Initializes a new project
* [install](install.md): Installs a global tool (alias)
* [lock](lock.md): Updates the lockfiles without installing dependencies
* [lint](lint.md): Run the linter on the project
* [make-req](make-req.md): Builds and prints a PEP 508 requirement string from parts
* [pin](pin.md): Pins a Python version to the project
* [publish](publish.md): Publish packages to a package repository
* [remove](remove.md): Remove a dependency from this project
* [run](run.md): Runs a command installed into this package
* [show](show.md): Prints the current state of the project
* [sync](sync.md): Updates the virtualenv based on the pyproject.toml
* [test](test.md): Runs the project's tests
* [toolchain](toolchain/index.md): Helper utility to manage Python toolchains
* [tools](tools/index.md): Helper utility to manage global tools.
* [self](self/index.md): Rye self management
* [uninstall](uninstall.md): Uninstalls a global tool (alias)
* [version](version.md): Get or set project version

## Options

The toplevel `rye` command accepts the following options:

* `--env-file` `<FILE>`: This can be supplied multiple times to make rye load
  a given `.env` file.  Note that this file is not referenced to handle the
  `RYE_HOME` variable which must be supplied as environment variable always.