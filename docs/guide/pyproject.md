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

##  `project.scripts`

This key specifies the scripts that are to be generated and installed into the virtual environment during `sync`.
These scripts will invoke the configured entry point.

```toml
[project.scripts]
my-hello-script = 'hello:main'
```
This configuration will generate a script `my-hello-script` that will call the `main` function of the
`hello` module.

Scripts can be installed using [`rye sync`](commands/sync.md) and run using [`rye run`](commands/run.md):

```bash
$ rye sync
$ rye run my-hello-script
Hello from hello!
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

## `tool.rye.generate-hashes`

+++ 0.35.0

When this flag is enabled all `lock` and `sync` operations in the project or workspace
operate as if `--generate-hashes` is passed.  This means that all dependencies in all
lock files will include a hash.

```toml
[tool.rye]
generate-hashes = true
```

## `tool.rye.lock-with-sources`

+++ 0.18.0

When this flag is enabled all `lock` and `sync` operations in the project or workspace
operate as if `--with-sources` is passed.  This means that all lock files contain the
full source references.  Note that this can create lock files that contain credentials
if the sources have credentials included in the URL.

```toml
[tool.rye]
lock-with-sources = true
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

## `tool.rye.virtual`

+++ 0.20.0

If this key is set to `true` the project is declared as a virtual project.  This is a special
mode in which the package itself is not installed, but only the dependencies are.  This is
for instance useful if you are not creating a Python project, but you are depending on Python
software.  As an example you can use this to install software written in Python.  This key is
set to true when `rye init` is invoked with the `--virtual` flag.

```toml
[tool.rye]
virtual = true
```

For more information consult the [Virtual Project Guide](../virtual/).

## `tool.rye.sources`

This is an array of tables with sources that should be used for locating dependencies.
This lets you use indexes other than PyPI.  These sources can also be configured in the
main `config.toml` config file with the same syntax.

```toml
[[tool.rye.sources]]
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

### `env-file`

+++ 0.30.0

This is similar to `env` but rather than setting environment variables directly, it instead
points to a file that should be loaded (relative to the `pyproject.toml`):

```toml
[tool.rye.scripts]
devserver = { cmd = "flask run --debug", env-file = ".dev.env" }
```

### `chain`

This is a special key that can be set instead of `cmd` to make a command invoke multiple
other commands.  Each command will be executed one after another.  If any of the commands
fails, the rest of the commands won't be executed and the chain fails.

```toml
[tool.rye.scripts]
lint = { chain = ["lint:black", "lint:flake8" ] }
"lint:black" = "black --check src"
"lint:flake8" = "flake8 src"
```

### `call`

This is a special key that can be set instead of `cmd` to make a command invoke python
functions or modules.  The format is one of the three following formats:

* `<module_name>`: equivalent to `python -m <module_name>`
* `<module_name>:<function_name>`: runs `<function_name>` from `<module_name>` and exits with the return value
* `<module_name>:<function_name>(<args>)`: passes specific arguments to the function

Extra arguments provided on the command line are passed in `sys.argv`.

```toml
[tool.rye.scripts]
serve = { call = "http.server" }
help = { call = "builtins:help" }
hello-world = { call = "builtins:print('Hello World!')" }
```

## `tool.rye.workspace`

When a table with that key is stored, then a project is declared to be a
[workspace](../workspaces/) root.  By default all Python projects discovered in
sub folders will then become members of this workspace and share a virtualenv.
Optionally the `members` key (an array) can be used to restrict these members.
In that list globs can be used.  The root project itself is always a member.

```toml
[tool.rye.workspace]
members = ["mylib-*"]
```

For more information consult the [Workspaces Guide](../workspaces/).
