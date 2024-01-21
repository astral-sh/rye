# Meta Server

*This document collects my notes on what a meta server might look like. That's not a fully fleshed
out proposal in itself.*

Today Python installers install packages from package repositories.  Typically this is `PyPI` but
really it can be almost anything that contains directory listing in HTML formats as the "simple"
index is basically defined to be some HTML structure that the installers parse.

The more I spent on Python packaging the more I think that this system has value, but really needs
an auxiliary system to function properly.  This document describes this system (referred to as
"Meta Server" or "metasrv"), why it should exist and how it could behave.

## Targeting

Today when installing packages, one configures an index.  For instance `https://pypi.org/simple/`.
If you have ever hit that URL you will have realized that it's an enormous HTML with every single
package very uploaded to PyPI.  Yet this is still in some sense the canonical way to install
packages.  If you for instance use `Rye` today you configure the index by pointing to that URL.

With the use of a **meta server**, one would instead point it to a meta server instead.  So for instance
the meta server for `pypi.org` could be hosted at a different URL, say `https://meta.pypi.org/`.
That meta server URL fully replaces the existing index URL.  Each meta server is supposed to target
a single index only.  A package manager _only_ interfaces with the meta server and it's the meta
server's responsibility to surfaces packages from the index it manages.  The index server is in this
proposal referred to as **source repository**.

## Purpose

The purpose of the meta server is manyfold:

* Expose an efficient index of all packages and versions hosted by the source repository.
* Cache original meta-data information from source archives and wheels
* Expose patched meta data information for resolvers (see note below)
* Accept "writes" from trusted parties to augment meta data entries (see note below)
* Maintain a list of well known marker values

The meta server can be self hosted, or it's hosted on behalf of a package management index.  It
by definition targets a single source repository.  For a company internal package index it for
instance would be possible to host the packages on an S3 bucket and to have a running meta server
fronting it.

### Efficient Index

It should be possible to replicate the index locally or to efficiently browse it partially via
a RESTful API.  The main lookup forms are:

1. Finding the canonical name of a package "foo" -> "Foo" if that's the registered name
2. Discovering all published versions of a package
3. Discovering the resolver relevant metadata

Note that the resolver relevant metadata might undergood patching.  That is to say that the
metadata is both exposed as stored in the wheel, but primarily exposed with manipulations
performed above.

The goal here is to also expose meta data from packages built from source so that a resolver does
not need to build source packages as part of resolving.

### Patched Meta Data

An installer and resolver is only useful if it's capable of installing the current state of the
Python world.  In practice there are packages that can be installed and combined with other
packages despite of their stated version ranges.  In particular upper bounds cause challenges
for packages today.  The goal for a meta server would be to accept patches to override these
dependencies after the publishing of a package.  As these overrides are unlikely to be shared
across the entire ecosystem, an idea is that these patches are local to an "understanding"
(see next section).

### Trusted Writes

For patched meta data the question comes up how such updates should be received.  In my mind the
source repository behind it represents the truth at the package publish time, whereas the meta
server reporesents the evolving understanding at a certain point in time from that point onwards.

There are three almost natural ways in which this changing understanding can evolve over time:

- ecosystems might develop a better understanding of dependency compatibility: for instance the
  pallets ecosystem might have a better understanding of which packages are in practice compatible
  with each other.  In a more complex world FastAPI and Pydantic might consider themselves a shared
  ecosystem and might develop a shared understanding of compatibility.
- there are people that might consider themselves auditors and might want to "notarize" packages
  before the even become available for installing.  They might want to add a layer of trust where
  they independently [audit packages](https://lucumr.pocoo.org/2016/3/24/open-source-trust-scaling/).
  As part of that auditing that might not just make packages available, they might also want to
  override meta data for better compatibility.
- the community as a hole might discover that some dependency bounds are too narrowly defined and
  express a desire to override it.

What all of these have in common are the following two aspects:

- there might not be consensus
- the understanding might change over time

I'm not sure what the right way is to approach this, but maybe the reality is that a meta server
might just have to roll with it and serve up different "understandings".  Maybe the most trivial
way would be that the meta server proxies more than one git repository that acts as the truth of
these patched meta data infos and users opt-into these via their installers.  Over time maybe some
understanding emerges which overrides are more appropriate.

So workflow wise the meta server might not directly accept writes, but it might become an arbiter
of where the writes should go.  So the "writes" here are really just virtual in the sense that
a tool might want to publish overwrites, but receives the information from the meta server where
these writes go (eg: which git repo) for a specific "understanding" they want to publish to.

An "understanding" in that sense is a freely defined thing that gets registered once with the
meta server.  For instance the meta server for `meta.pypy.org` could register the "pallets"
understanding.  A hypothetical `tool upload-override --package Flask --version 2.0 --file metadata.json --understanding pallets`
would receive from the meta server the location of a git repository where that metadata file
should be placed.  The user when installing would opt into one or multiple understandings which
are reified locally.

### Known Marker Values

In addition to good meta data, a resolver in Python also needs a better understanding of which
markers exist and are worth supporting.  To understand the motivation here: lock files ideally
contain enough information to not just install for your local machine, but also other versions
of Python or windows versions.  However actually doing so requires knowledge of what else is out
there.

There might be blessed sets of marker values that can be discovered via the meta server.  As an
example there might be a set of marker values called `linux_macos_windows-36m` which holds all
marker values for linux, macos and windows for supported Python versions that cover the last 36
months.
