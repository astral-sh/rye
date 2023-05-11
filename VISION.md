# A Vision of Rye

This document describes of what I envision Python packaging and project management
could look like.

## The Rust Experience

Coming from a Rust environment there are two tools which work together: `rustup` and
`cargo`.  The first one of those is used to ensure that you have the correct Rust
toolchain on your machine.  Rust greatly prefers binary distributions of the language
from the official website over external distributions.

`cargo` is the main entry point to development in Rust.  It acts as the tool to
trigger test runs, start the build process, shell out to the documentation building
tool, linters but also things such as workspace management, dependency management and
package publishing.

Crucially a very important aspect of the Rust development experience is the strong
commitment to semver and the built-in support for it.  This goes very deep.  The
resolver for instance will deduplicate matching dependencies throughout the graph.
This means that if four libraries depend on `libc@0.2`, they will all resolve to
that dependency.  However if another need arises for `libc@1.0`, then it's possible
for the dependency graph to result in both being loaded!

The ecosystem greatly depends on this.  For instance when a new major release is made
of a very core library, in some cases extra care is taken to unify the now incompatible
versions by re-exporting core types from the newer to the older version.  Thus it's
for instance possible for `important-lib@0.2.32` to depend on
`important-lib@1.0` internally so it can make the transition easier.

Additionally Rust heavily leverages lockfiles.  Whenever you compile, the dependencies
are locked in place and future builds reuse the same dependency versions unless you
update.

Most importantly though the Rust ecosystem has embraced `rustup` and `cargo` that the
vast majority of people are using these tools on a daily basis.  Even developers who
pick other tools like buck, are still using `cargo` regularly.

## Going Python

Rye wants to explore if such an experience is possible with Python.  I believe it can!
There is quite a lot of the ecosystem that can be leveraged for this purpose but there
is even more that would need to be built.

**Important note:** when you read "rye" in the context of the document it talks about
what a potential tool like rye could be.  It might as well be that one of the many
tools that exist today, turn into that very tool that is described here.

My sentiment is that unless "the one tool" can emerge in the Python world, the
introduction of yet another tool might be a neg-negative to the ecosystem.  Plenty of
tools have been created over the years, and unfortunately it hasn't been able to
rally the majority of the Python community behind any tool.  I do however believe it is
possible.

### Bootstrapping Python

I believe the right approach is that >95% of users get a Python distribution via `rye`
and not to have `rye` pick up a system installed Python distribution.  There are good
reasons for using a system Python installation, but it should be the exception not the
rule.  Most importantly because a Python distribution that `rye` puts in place can be
made to have reliable and simple rules that do not differ between systems.

A huge cause of confusion and user frustration currently comes from Linux distribution
specific patches on top of Python that break tools and change behavior, particularly
in the python packaging ecosystem.

Bootstrapping Python via an independent tool has other benefits as well.  It for instance
allows much easier cross-python version testing via tox or CI.

**What needs to be done:**

* Provide widely available Python builds, with largely standardized structure
  retrievable from the internet. [PEP 711](https://peps.python.org/pep-0711/) is a step
  in that direction.

### A Stronger Resolver

Today there are a ton of different resolvers in the Python ecosystem.  Pip has two, poetry
has one, pdm has one, different independent Python and Rust resolvers exist on top of that.
Resolvers are important, but unfortunately are are both too many and too many issues with
the existing ones.  Here is what I believe a resolver needs to be able to accomplish:

* **Allow resolving across markers:** most resolvers in the Python ecosystem today can only
  resolve for the current interpreter and platform (eg: pip, pip-tools).  This means it cannot
  create a resolution that is equally valid for a different platform.  In parts this is
  a problem because of how environment markers in Python are defined.  They allow a level of
  expressiveness that cannot be reflected by most tools, however a subset could be supported.

* **Multi-version resolution support:** this is a bit foreshadowing, but I believe for a
  variety of reasons it needs to be possible for a resolver to not unify all requirements
  to a single version, but to support multiple independent resolutions across major versions
  of libraries.  A future resolver should be able to permit `package==2.0` and `package==1.1`
  to both be resolved for different parts of the tree.

* **Resolver API:** access to the resolver is important.  For editor plugins, or custom
  tools it's always necessary to be able to resolve packages.  For instance if you want
  something as trivial as "add latest supported version of 'flask' to my `pyproject.toml`"
  you need to be able to work with the resolver.

* **Filters:** I strongly believe that a good resolver also needs a filter on top.  For
  instance it must be possible for a developer to restrict the resolver to stay within the
  bounds of the target Python version and to never upgrade into a tree containing Python
  versions that are too new.  Likewise for supply chain safety a resolver should be able to
  restrict itself to a set of vetted dependencies.

**What needs to be done:**

* Create a reusable resolver that can be used by multiple tools in the ecosystem.
* Make the resolver work with the proposed metadata cache
* Expose the resolver as API for multiple tools to use.
* Add a policy layer into the resolver that can be used to filter down the dependencies
  before use.

### Metadata Caches

Because of the rather simplistic nature of Python packages and package indexes a resolver
will always be restricted by the metadata that it can reliably pull.  This is particularly
bad if the system needs to fall back to `sdist` uploads which in the worst case requires
executing python code to determine the dependencies, and those dependencies might not even
match on different platforms.

However this is a solvable problem with sufficient caching, and with the right design for
the cache, this cache could be shared.  It might even be quite interesting for PyPI to
serve up "fake" metadata records for popular sdist only packages to help resolvers. 
This might go a long way in improving the quality of the developer experience.

**What needs to be done:**

* Local metadata caches are added for the resolver to use
* PyPI gains the ability to serve dependency meta data

### Lockfiles

It's unclear if a standard can emerge for lock files given the different requirements, but a
Python packaging solution needs to have support for these.  There are a lot of different
approaches to lockfiles today (poetry and pdm for instance have them) but it's not entirely
clear to me that the way they are handled today is sufficiently pragmatic to enable a tool
that is based on lockfiles to get majority adoption.

The reason in part relates the suboptimal situation with resolvers (eg: large projects can
take ten minutes or longer to dependency check in poetry), on the other hand however also
because of the reality of how dependencies are currently declared.  For instance certain
libraries will "over" depend on third party libraries, even if they are not needed for a
developer.  These pulled in dependencies however will still influence the resolver.

Most importantly a good lockfile also covers platforms other than the current developer's
machine.  This means that if a project supports Windows and Linux, the lockfile should be
handling either dependency trees.  This is what cargo accomplishes today, but cargo has a
a much simpler problem to solve here because it has perfect access to package metadata which
resolvers in Python do not have today.  What is also problematic in Python is that certain
parts of the dependency tree can be version dependent.  In Rust a library A either depends
on library B or it does not, but it does not depend on it conditional to a Python version.

The total expressiveness of Python dependencies is challenging.  The lack of good metadata
access for the resolver combined with the ability to make dependencies optional conditional
to the Python version is tricky by itself.  The complexity however is compounded by the
fact that the resolver needs to come to a solution that can only result in a single resolved
version per package.

**What needs to be done:**

* Experiment with a restricted lock format that satisfies a subset of what markers provide
  today, that strikes a good balance.
* Provide lockfile support as part of the resolver library.

### Upper Bounds & Multi Versioning

Resolving Python dependencies is particularly challenging because a single solution must be
found per package.  A reason this works at all in the Python ecosystem is that most libraries
do not set upper bounds.  This means that they will be eagerly accepting future libraries even
at the cost of not supporting them.  That's largely possible because Python is a dynamic
language and a lot of flexibility is usually possible here.  However with increased utilization
of type information in the Python world, and maybe with stronger desires for proper locking,
it might be quite likely that upper version bounds become more common.

Once that happens however, the Python ecosystem will quite quickly run into blocking future
upgrades until the entire dependency graph has moved up which creates a lot of friction.
Other ecosystems have solved this problem by strictly enforcing semver semantics onto packages
and by permitting multiple semver incompatible libraries to be loaded simultaneously.  While
usually a library is only allowed to permit on a single version of a dependency, that dependency
can exist in different versions throughout the dependency tree.

In Python there is a perceived worry that this cannot be accomplished because of how site-packages,
`PYTHONPATH` and `sys.modules` works.  However I believe these to be solvable issues.  On the one
hand because `.pth` files can be used to completely change how the import system works, secondly
because the `importlib.metadata` API is strong enough these days to allow a package to resolve
it's own metadata.  The combination of the two can be used to "redirect" imports in `sys.modules`
and import statements to ensure that if a library imports a dependency of itself, it ends up with
the right version.

**What needs to be done:**

* Add a new metadata key to `pyproject.toml` that declares that a package supports multi-versioning
* Enforce semver semantics on multi-version dependencies
* Provide an import hook that provides multi-version imports as part of Rye
* Relax the resolver to permit multiple solutions for multi-version dependencies

### Workspaces and Local / Multi Path References

With growing development teams one of the most frustrating experiences is the inability to
break up a monolithic Python module into smaller modules without having to constantly publish
minor versions to a package index.  The way the Rust ecosystem deals with this issue is two-fold:
one the one hand Rust supports workspaces natively.  Workspaces share dependencies and the
resolver results.  The equivalent in Python would be that a workspace shares a virtualenv
across all of the projects within in.  The second way in which Rust solves this problem is
to permit a dependency to both support declaration of the package name, index but also local
reference.

While also Rust does not permit a crate to be published to a package index with references to
packages outside of the index, a separate rewrite step kicks in ahead of publish to clean out
invalid dependency references.  If no valid reference remains, the package will not publish.

**What needs to be done:**

* requirement declarations need to be expanded to support defining the name of the index where
  they can be found, and optional local path references.

### Every Project in a Virtualenv

While virtualenv is not by favorite tool, it's the closest we have to a standard.  I proposed
that there is always one path for a virtualenv `.venv` and when Rye manages it, users should
not interact with it manually.  It's at that point rye's responsibility to manage it, and it
shall manage it as if it was a throw-away, always re-creatable scratch-pad for dependencies.

Preferably over time the structure of virtualenvs aligns between different Python versions
(eg: Windows vs Linux) and the deeply nested `lib/py-ver/site-packages` structure is flattened
out.

**What needs to be done:**

* Agree on a name for where managed virtualenvs are placed (eg: `.venv` in the workspace root)

### Dev and Tool Dependencies

Another topic that is currently unresolved across tools in the ecosystem is how to work with
dependencies that are not used in production.  For instance it's quite common that a certain
dependency really only matters on the developer's machine.  Today pdm and some other tools
have custom sections in the `pyproject.toml` file to mark development dependencies, but there
is no agreement across tools on it.

**What needs to be done:**

There needs to be an agreed upon standard for all tools.  [See this discussion](https://discuss.python.org/t/development-dependencies-in-pyproject-toml/26149/7)

### Opinionated Defaults

Python against PEP-8's wishes just has too many ways in which things can be laid out.  There
should be a much stronger push towards encouraging common standards:

**What needs to be done:**

* Rye shall ship with the one true formatter
* Rye shall ship with the one true linter
* Rye shall always create a preferred folder structure for new projects
* Rye shall loudly warn if `package-foo` does not provide a `package_foo` module

## Existing Tools

Some of the existing tools in the ecosystem are close, and there is a good chance that some
of these might be able to combine forces to create that one-true tool.  I hope that there
is enough shared interest, that we don't end up with three tools that all try to be Rye.
