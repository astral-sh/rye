# Workspaces

Workspaces are a feature that allows you to work with multiple packages that
have dependencies to each other.  A workspace is declared by setting the
`tool.rye.workspace` key a `pyproject.toml`.  Afterwards all projects within
that workspace share a singular virtualenv.

## Declaring Workspaces

A workspace is declared by the "toplevel" `pyproject.toml`.  At the very least
the key `tool.rye.workspace` needs to be added.  It's recommended that a glob
pattern is also set in the `members` key to prevent accidentally including
unintended folders as projects.

```toml
[tool.rye.workspace]
members = ["myname-*"]
```

This declares a workspace where all folders starting with the name `myname-`
are considered.  If the toplevel workspace in itself should not be a project,
then it should be declared as a virtual package:

```toml
[tool.rye]
virtual = true

[tool.rye.workspace]
members = ["myname-*"]
```

For more information on that see [Virtual Packages](../virtual/).

## Syncing

In a workspace it does not matter which project you are working with, the entire
workspace is synchronized at all times.  This has some untypical consequences but
simplifies the general development workflow.

When a package depends on another package it's first located in the workspace locally
before it's attempted to be downloaded from an index.  The `--all-features` flag is
automatically applied to all packages, but to turn on the feature of a specific
package the feature name must be prefixed.  For instance to enable the `foo` extra feature
of the `myname-bar` package you would need to do this:

```
rye sync --features=myname-bar/foo
```
