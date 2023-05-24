# Python Project (`pyproject.toml`)

Rye tries to avoid a lot of proprietary configuration in the `pyproject.toml` file but a bit
is necessary.  Here are the most important keys that Rye expects:

## `project.dependencies`

This key is used to manage dependencies.  They work exactly like you expect from a regular
`pyproject.toml` file and in fact Rye changes nothing about this.  However Rye is capable
of modifying these entries with the `rye add` and `rye remove` commands.

```toml
[project]
dependencies = [
    "mkdocs~=1.4.3",
    "mkdocs-material~=9.1.12",
    "pymdown-extensions~=9.11",
]
```

## `tool.rye.dev-dependencies`

This works similar to `project.dependencies` but holds development only dependencies.  These
can be added here automatically via `rye add --dev`.

```toml
[tool.rye]
dev-dependencies = ["black~=23.3.0"]
```

Dev dependencies are installed automatically unless `--no-dev` is passed to `sync`.

## `tool.rye.excluded-dependencies`

This is a special key that contains dependencies which are never installed, even if they are
pulled in as indirect dependencies.  These are added here automatically with `rye add --excluded`.

```toml
[tool.rye]
excluded-dependencies = ["cffi"]
```

## `tool.rye.managed`

+++ 0.3.0

This key tells rye that this project is supposed to be managed by Rye.  This key
primarily affects some automatic creation of virtualenvs.  For instance Rye
will not try to initialize a virtualenv when using shims without this flag.  It
can be forced enabled in the global config.

```toml
[tool.rye]
managed = true
```

## `tool.rye.sources`

This is an array of tables with sources that should be used for locating dependencies.
This lets you use indexes other than PyPI.  These sources can also be configured in the
main `config.toml` config file with the same syntax.

```toml
[[sources]]
name = "default"
url = "http://pypi.org/simple/"
```

For more information about configuring sources see [Dependency Sources](sources.md).

## `tool.rye.scripts`

This key can be used to register custom scripts that are exposed via `rye run`.  Each key is
a script, and each value is the configuration for that script.  Normally the value is an object
with different keys with the most important key being `cmd` which holds the command to execute.
However if only `cmd` is set, then the object is optional.  `cmd` itself can either be set to a
string or an array of arguments.

```toml
[tool.rye.scripts]
# These three options are equivalent:
devserver = "flask run --app ./hello.py --debug"
devserver-alt = ["flask", "run", "--app", "./hello.py", "--debug"]
devserver-explicit = { cmd = "flask run --app ./hello.py --debug" }
```

The following keys are possible for a script:

### `cmd`

The command to execute.  This is either a `string` or an `array` of arguments.  In either case
shell specific interpolation is unavailable.  The command will invoke one of the tools in the
virtualenv if it's available there.

```toml
[tool.rye.scripts]
devserver = { cmd = "flask run --app ./hello.py --debug" }
http = { cmd = ["python", "-mhttp.server", "8000"] }
```

### `env`

This key can be used to provide environment variables with a script:

```toml
[tool.rye.scripts]
devserver = { cmd = "flask run --debug", env = { FLASK_APP = "./hello.py" } }
```

### `chain`

This is a special key that can be set instead of `cmd` to make a command invoke multiple
other commands.  Each command will be executed one after another.  If any of the commands
fails the rest of the commands won't be executed and instead the chain fails.

```toml
[tool.rye.scripts]
lint = { chain = ["lint:black", "lint:flake8" ] }
"lint:black" = "black --check src"
"lint:flake8" = "flake8 src"
```

## `tool.rye.workspace`

When a table with that key is stored, then a project is declared to be a workspace root.  By
default all Python projects discovered in sub folders will then become members of this workspace
and share a virtualenv.  Optionally the `members` key (an array) can be used to restrict these
members.  In that list globs can be used.  The root project itself is always a member.

```toml
[tool.rye.workspace]
members = ["mylib-*"]
```

