# Markers or Locking

*This document collects my notes on locking. That's not a fully fleshed out proposal in itself.*

One of the largest challenges with creating lock files is that there is a desire to make lock files
that are portable.  Portable means that the lock file should incorporate enough information that it
can be taken from one computer to the next one, and the lock file can be re-used even if the Python
version changed or the operating system is a different one.

## General Challenges

There are a handful of basic challenges with portable lock files.  These are largely independent of
the "marker problem" that this document describes, but they are important to understand nonetheless

* source distributions can have unstable dependencies.  This means that for instance running `setup.py`
  on one machine might produce dramatically different version dependencies than running this on another
  machine.  They are in that sense in some cases **not using markers**.
* wheels might have conflicting version dependencies.  This can be a result of the previous bullet, or
  as a deliberate choice.  That means that even for the same package version there are wheels which have
  different dependencies for different platforms.
* Python has no general understanding of compatibility of packages on the version level.  Unlike for
  instance the node or rust ecosystem semver is not encoded into version numbers which means that the
  resolver cannot leverage this.  This also means that the ecosystem uses a lot of upper bounds.

## Known Marker Values

Environment markers exist in all kinds of flavors.  They describe under which circumstances a dependency
must be used.  This can be done in quite extreme manners.  For instance the following is a valid
dependency declaration:

```toml
[project]
dependencies = [
    'more-itertools>=4.0.0,<6.0.0;python_version<="2.7"',
    'more-itertools>=4.0.0;python_version>"2.7"',
]
```

And pip lets you have requirements files like this:

```
"awesome_package @ https://example.com/awesome_package-cp39-cp39-linux_x86_64.whl ; sys_platform == 'linux' and python_version == '3.9'",
"awesome_package @ https://example.com/awesome_package-cp310-cp310-linux_x86_64.whl ; sys_platform == 'linux' and python_version == '3.10'",
"awesome_package @ https://example.com/python/awesome_package-cp39-cp39-macosx_12_0_x86_64.whl ; sys_platform == 'darwin' and python_version == '3.9'",
"awesome_package @ https://example.com/python/awesome_package-cp310-cp310-macosx_12_0_x86_64.whl ; sys_platform == 'darwin' and python_version == '3.10'",
```

It looks innocent but it basically "splits" the space in half about which packages need to be considered
for which Python version.  In a sense observing markers "expands" the space of potential solutions that
need to be considered.  If one only encounters a dependency without anything other than a lower bound
and no extra marker, the problem stays quite contained.  But then one might encounter for the first time
a `python_version` marker and all the sudden the solutions would need to be found for ever possible
Python version that exists.  Likewise the first time you come across a platform marker for
`sys_platform == 'darwin'` one would have to start going down that route as well.

The most obvious solution would be to observe ever marker value that comes by, and to add it to the
final result set.  That however potentially means that the total set of dependencies to consider is
excessive.  It might also require one going down the path of evaluating a bunch of `sdist` distributions
that require building in hope of encountering more metadata.

I believe the way PDM restricts the search space is by requiring the set of `python_version`\s that
should be considered to be configured.  There are however quite a few potential complications still.
For instance it's acceptable to have something like `python_version<="3.8.1"` (eg: minor versions).
PDM also [collapses some markers together](https://github.com/pdm-project/pdm/issues/46) resolving in
incorrect lockfiles.

### Goal Setting

I believe that markers need to be supported, but they probably should be restricted for the following
three goals:

1. fast resolving: reduce the total set of versions to be considered early
2. common cases: support common marker configurations that actually happen
3. finding a solution: the resolver should result in a solution, even if that solution might be a "bit" wrong.

The third point probably requires a bit of explanation: today many packages are only installable by
lying about version constraints somewhere.  In parts that's because packages define upper bounds of
dependencies in anticipation of future incompatibility that might not even exist.

### Reducing The Problem Space

In some sense a potential option is to just have a fully exploded set of permutations of markers and then
create resolutions for all those.  At least for the combination of Python version and operating system
that might even work, if Python versions are limited to major versions (or a specific minor version
is targeted).

An appealing option would be to just only use Python version ranges (which are likely to be required
anyways by the root package) and to disregard all platform markers for the resolution.  Only at an
installation time would the platform markers come into play.  I believe this model to work somewhat,
but there are definitely challenges.  First of all this model would need to take disjoint markers into
account where different platforms might demand different packages.  This not only has an effect on that
package, but on the total set of packages to consider for resolution as each of those packages can
pull in further dependencies.  In a sense, the resolver would "fork" whenever it encounters conflicting
constraints on dependencies.  This can quickly explode in complexity and that's an issue that poetry
is frequently running into in scientific Python ecosystem (see
[Multiple constraints on same package cause O(exp(N)) checks](https://github.com/python-poetry/poetry/issues/5121)).

I believe the only real solution is to dramatically reduce the total number of permutations that can
be considered.  This reduction in permutations I think can only come from dramatically merging versions
eagerly.

As an extreme example this is what `opencv-python` likes to declare with markers:

```
numpy >=1.13.3 ; python_version < "3.7"
numpy >=1.21.0 ; python_version <= "3.9" and platform_system == "Darwin" and platform_machine == "arm64"
numpy >=1.21.2 ; python_version >= "3.10"
numpy >=1.21.4 ; python_version >= "3.10" and platform_system == "Darwin"
numpy >=1.23.5 ; python_version >= "3.11"
numpy >=1.19.3 ; python_version >= "3.6" and platform_system == "Linux" and platform_machine == "aarch64"
numpy >=1.17.0 ; python_version >= "3.7"
numpy >=1.17.3 ; python_version >= "3.8"
numpy >=1.19.3 ; python_version >= "3.9"
```

Even reducing the total space of supported Python versions is still a lot.  Maybe a user just does not
care about macOS, in which case it's not helpful considering those constraints.  Likewise there is a not
insignificant chance that the root application is untested/does not support arm64.  What's also worth
pointing out here is that `platform_machine` doesn't have consistent values.  For instance an aarch64 CPU
can be identified by `arm64` on macOS but `aarch64` on Linux.

The easiest way to reduce the space would be to be strict about pruning part of the resolved space that
is irrelevant in general, or at least irrelevant to the project.  So any version constraint lower than
what is the lower bound of the root library can be removed entirely.  Likewise during resolution it might
be possible to make a pass over the absolute lowest versions supported of a dependency to reduce the
search space.

Instead of asking a user to manually restrict the problem space, it might be reasonable to work
with known ecosystem markers.  Eg: this project targets mac + linux + windows of release versions
of libraries in the last 12 months.  See also some notes in [`metasrv`](metasrv.md).
