# Dependencies

Dependencies are declared in [pyproject.toml](pyproject.md) however adding them can be
simplified with the `rye add` command.  In the most simple invocation it adds a regular
dependency, but it can be customized.

## Adding Basic Dependency

To add a regular dependency just invoke `rye add` with the name of the Python package:

```
rye add Flask
```

If you also want to define a version, use a [PEP 508](https://peps.python.org/pep-0508/)
requirement:

```
rye add "Flask>=2.0"
```

For extra/feature dependencies you can either use PEP 508 syntax or use `--features`:

```
rye add "Flask[dotenv]"
rye add Flask --features=dotenv
```

These dependencies are stored in [`project.dependencies`](pyproject.md#projectdependencies).

!!! tip "Note about pre-releases"

    By default `add` will not consider pre-releases.  This means if you add a dependency
    that has `.dev` or similar in the version number you will not find a match.  To
    consider them, add them with `--pre`:

    ```
    rye add "Flask==2.0.0rc2" --pre
    ```

## Development Dependencies

For dependencies that should only be installed during development pass `--dev`

```
rye add --dev black
```

These dependencies are stored in the non-standard
[`tool.rye.dev-dependencies`](pyproject.md#toolryedev-dependencies) key.

## Git / Local Dependencies

To add a local or git dependency, you can pass additional parameters like `--path`
or `--git`:

```
rye add Flask --git=https://github.com/pallets/flask
rye add My-Utility --path ./my-utility
```

Note that when adding such dependencies, it's necessary to also provide the name
of the package.  Additionally for git dependencies all kinds of extra parameters
such as `--tag`, `--rev` or `--branch` are supported.

When working with local dependencies it's strongly encouraged to configure a
[workspace](pyproject.md#toolryeworkspace).
