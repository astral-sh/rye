# Toolchain Management

Rye is unique in that it does not use system Python installations.  Instead it downloads
and manages Python installations itself (called toolchains).  Today there are
three types of toolchains supported by Rye and they require some understanding:

* [**Portable CPython**](cpython.md): Rye will itself download portable builds of CPython
  for most of its needs.  These are fetched from
  [indygreg/python-build-standalone](https://github.com/indygreg/python-build-standalone)
* [**Official PyPy Builds**](pypy.md): PyPy is supported from the official release builds.
* [**Custom Local Toolchains**](#registering-toolchains): locally installed Python interpreters can be
  registered with Rye.  Afterwards, they can be used with any Rye managed project.

## Pinning Toolchains

To make a project use a specific toolchain write the name of the toolchain into the
`.python-version` file or use the `pin` command.  For pinning `cpython` the `cpython@`
prefix can be omitted.

```
rye pin cpython@3.11.4
```

Pinning a downloadable version means that Rye will automatically fetch it when necessary.
By default, toolchains are pinned to a precise version.  This means that even if you
write `rye pin cpython@3.11`, a very specific version of cpython is written into the
`.python-version` file.  With Rye 0.5.0 onwards it's possible to perform "relaxed" pins:

```
rye pin --relaxed cpython@3.11
```

This will then persist `3.11` in the `.python-version` file and Rye will use the latest
available compatible version for the virtual environment.

+/- 0.5.0

    Relaxed pinning with `rye pin --relaxed` was added.

## Non Native Architectures

+++ 0.14.0

    Support for fetching and pinning of non-native architectures was added.

By default, the pin is for the architecture of the running machine.  This means that
if you pin `cpython@3.11` on a mac with aarch64 architecture, you will use a cpython
interpreter of that CPU architecture.  A different architecture can be selected by
adding `-{arch}` to the python family name.  So for instance to force a `x86_64` version
you need to pin like this:

```
rye pin cpython-x86_64@3.11
```

Note that such custom pins are not reflected in `pyproject.toml` but only `.python-version`.

## Listing Toolchains

To see which toolchains are installed, `rye toolchain list` prints a list:

```
rye toolchain list
```
```
cpython@3.11.1 (C:\Users\armin\.rye\py\cpython@3.11.1\install\python.exe)
pypy@3.9.16 (C:\Users\armin\.rye\py\pypy@3.9.16\python.exe)
```

To see which toolchains can be installed, additionally pass the `--include-downloadable`:

```
rye toolchain list --include-downloadable
```

## Fetching Toolchains

Generally Rye automatically downloads toolchains, but they can be explicitly fetched
with `rye toolchain fetch` (also aliased to `rye fetch`):

```
rye toolchain fetch cpython@3.8.5
```

Starting with Rye 0.19.0 the argument to `fetch` is inferred from the current pin.  This means
you can also fetch as follows:

```
rye pin 3.10
rye fetch
```

Toolchains are fetched from two sources:

* [Indygreg's Portable Python Builds](https://github.com/indygreg/python-build-standalone) for CPython
* [PyPy.org](https://www.pypy.org/) for PyPy

## Registering Toolchains

Additionally, it's possible to register an external toolchain with the `rye toolchain register`
command.

```
rye toolchain register /path/to/python
```

The name of the toolchain is picked based on the interpreter.  For instance
linking a regular cpython installation will be called `cpython@version`, whereas
linking pypy would show up as `pypy@version`.  From Rye 0.5.0 onwards `-dbg` is
appended to the name of the toolchain if it's a debug build.  To override the
name you can pass `--name`:

```
rye toolchain register --name=custom /path/to/python
```

## Removing Toolchains

To remove an already fetched toolchain run `rye toolchain remove`.  Note that this
also works for linked toolchains:

```
rye toolchain remove cpython@3.8.5
```

!!! Warning

    Removing an actively used toolchain will render the virtualenvs that refer to use broken.
