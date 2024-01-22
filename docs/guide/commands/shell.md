# `shell`

Spawns a shell with the virtualenv activated.

**Warning:** this feature is inherently buggy as shells do not support portable APIs
to enable this functionality.  This command might to away if it cannot be fixed.

## Example

Run a tool from the virtualenv:

```
$ rye shell
Spawning virtualenv shell from /Users/username/my-project/.venv
Leave shell with 'exit'
```

To exit the sub shell run `exit`.

## Arguments

*no arguments*

## Options

* `--no-banner`: Do not show banner

* `--allow-nested`: Allow nested invocations

* `--pyproject`: Use this `pyproject.toml` file

* `-h, --help`: Print help (see a summary with '-h')