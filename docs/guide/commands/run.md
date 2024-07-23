# `run`

Runs a command installed into this package.  This either runs a script or application
made available in the virtualenv or a Rye specific script.  If no command is provided, it will list all available commands.

If there is a script in the virtualenv having the same name as a Rye script, the virtualenv script will take precedence over the Rye script.

For more information see [`rye.tool.scripts`](../pyproject.md#toolryescripts).

## Example

Run a tool from the virtualenv:

```
$ rye run flask
```

Invoke it without arguments to see all available scripts:

```
$ rye run
flask
hello
python
python3
python3.9
```

## Arguments

* `[COMMAND]`: The name of the command and the arguments to it.

## Options

* `-l, --list`: List all commands (implied without arguments)

* `--pyproject`: Use this `pyproject.toml` file

* `-h, --help`: Print help (see a summary with '-h')