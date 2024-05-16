# Virtual Projects

+++ 0.20.0

Virtual projects are projects which are themselves not installable Python
packages, but that will sync their dependencies.  They are declared like a
normal python package in a `pyproject.toml`, but they do not create a package.
Instead the `tool.rye.virtual` key is set to `true`.

For instance this is useful if you want to use a program like `mkdocs` without
declaring a package yourself:

```
rye init --virtual
rye add mkdocs
rye sync
rye run mkdocs
```

This will create a `pyproject.toml` but does not actually declare any python code itself.
Yet when syncing you will end up with mkdocs in your project.

## Behavior Changes

When syncing the project itself is never installed into the virtualenv as it's not
considered to be a valid package.  Likewise you cannot publish virtual packages to
PyPI or another index.

### Run tests outside of application code

If you tests are separated from your application code (typically by putting them in a
`tests` folder next to the `src` folder) you need to add the `src` folder to the python
path. With pytest you can achieve this with the following section in your `pyproject.toml`
file:
```toml
[tool.pytest.ini_options]
pythonpath = "src"
```

## Limitations

Virtual projects can not have optional dependencies.  These even if declared are not
installed.

## Workspaces

If a [workspace](../workspaces/) does not have a toplevel package it's
recommended that it's declared as virtual.
