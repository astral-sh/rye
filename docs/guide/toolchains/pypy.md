# PyPy

[PyPy](https://www.pypy.org/) is supported as alternative Python distribution.
Like the portable CPython builds it's downloaded automatically.  The name for
PyPy distributions is `pypy`.

## Limitations

PyPy has some limitations compared to regular Python builds when it comes to
working with Rye.  Most specifically PyPy uses some internal pypi dependencies
and you might notice warnings show up when syching.  PyPy also lags behind
regular Python installations quite a bit these days so you likely need to
target older Python packages.

## Sources

PyPy builds are downloaded from
[downloads.python.org](https://downloads.python.org/pypy/).

## Usage

When you pin a Python version to `pypy@major.minor.patch` then Rye will
automatically download the right version for you whenever it is needed.  If a
[custom toolchain](index.md#registering-toolchains) has already been registered
with that name and version, that this is used instead.  Note that the version
refers to the PyPy **CPython** version.

That means for instance that PyPy 7.3.11 is identified as `pypy@3.9.16` as this
is the Python version it provides.  As PyPy also lacks builds for some CPU
architectures, not all platforms might provide the right PyPy versions. 
