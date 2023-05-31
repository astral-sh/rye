# Portable CPython

Rye is capable (and prefers) to download its own Python distribution over what
you might already have on your computer.  For CPython, the
[indygreg/python-build-standalone](https://github.com/indygreg/python-build-standalone)
builds from the PyOxidizer project are used.

The motivation for this is that it makes it easy to switch between Python
versions, to have a common experience across different Rye users and to
avoid odd bugs caused by changes in behavior.

Unfortunately Python itself does not release binaries (or the right types of
binaries) for all operating systems which is why Rye leverages the portable
Python builds from PyOxidizer.

Unlike many other Python versions you can install on your computer are
non-portable which means that if you move them to a new location on your
machine, or you copy it onto another computer (even with the same operating
system) they will no longer run.  This is undesirable for what Rye wants to do.
For one we want the same experience for any of the Python developers, no matter
which operating system they used.  Secondly we want to enable self-contained
Python builds later, which requires that the Python installation is portable.

To achieve this, the Python builds we use come with some changes that are
different from a regular Python build.

## Limitations

The following changes to a regular Python versions you should be aware of:

* `libedit` instead of `readline`: unfortunately `readline` is GPL2 licensed
  and this is a hazard for redistributions.  As such, the portable Python
  builds link against the more freely licensed `libedit` instead.

* `dbm.gnu` is unavailable.  This is a rather uncommonly used module and the
  standard library provides alternatives.

Additionally due to how these builds are created, there are some other quirks
you might run into related to terminal support or TKinter.  Some of these
issues are collected in the [FAQ](../faq.md).  Additionally the Python
Standalone Builds have a [Behavior Quirks](https://python-build-standalone.readthedocs.io/en/latest/quirks.html)
page.

## Sources

Portable CPython builds are downloaded from GitHub
([indygreg/python-build-standalone/releases](https://github.com/indygreg/python-build-standalone/releases))
and SHA256 hashes are generally validated.  Some older versions might not
have hashes available in which case the validation is skipped.

## Usage

When you pin a Python version to `cpython@major.minor.patch` (or just
`major.minor.patch`) then Rye will automatically download the right version
for you whenever it is needed.  If a [custom toolchain](index.md#registering-toolchains) has already been registered with that name and
version, that this is used instead.
