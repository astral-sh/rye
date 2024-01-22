# `register`

Register a Python binary as custom toolchain.

Rye by default will automatically download Python releases from the internet.
However it's also possible to register already available local Python
installations.  This allows you to use rye with self compiled Pythons.

The name of the toolchain is auto detected (eg: `cpython`, `pypy` etc.)

To unregister use the [`remove`](remove.md) command.

## Example

```
$ rye toolchain register /opt/homebrew/Cellar/python@3.10/3.10.6_1/bin/python3.10
Registered /opt/homebrew/Cellar/python@3.10/3.10.6_1/bin/python3.10 as cpython@3.10.6
```

## Arguments

* `<PATH>`: Path to the python binary that should be registered

## Options

* `-n, --name <NAME>`: Name of the toolchain.  If not provided a name is auto detected.

* `-h, --help`: Print help (see a summary with '-h')