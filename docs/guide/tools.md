# Tools

Rye supports global tool installations.  This for instance allows you to install
tools like `black` or `ruff` globally.

## Installing Tools

Use the `rye tools install` (aliased to `rye install`) command to install a tool
globally with a shim:

```bash
rye install ruff
```

Afterwards the tool is installed into `~/.rye/tools/ruff` and the necessary shims
are placed in `~/.rye/shims`.

+/- 0.4.0

    The `install` command now considers custom sources configured
    in the `config.toml` file.  For more information see [Dependency Sources](sources.md).

## Listing Tools

If you want to see which tools are installed, you can use `rye tools list`:

```
rye tools list
```

```
black
  black
  blackd
ruff
  ruff
```

To also see which scripts those tools provide, also pass `--include-scripts`

```
rye tools list --include-scripts
```

## Uninstalling Tools

To uninstall a tool again, use `rye tools uninstall` (aliased to `rye uninstall`):

```
rye uninstall black
```
