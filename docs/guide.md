# User Guide

> ⚠️ Note: `rye` is in an experimental state.

## Contents

You can find a demo of `rye` [here](https://youtu.be/CyI8TBuKPF0).

* [Getting started](#getting-started)
* [Manage your Python toolchain](#manage-your-python-toolchain)
* [Start a project](#start-a-project)
* [Change your current Python version](#change-your-current-python-version)
* [Add a dependency to your project](#add-a-dependency)
* [Sync your Python environment with your project](#sync-your-python-environment)
* [Remove a dependency from your project](#remove-a-dependency)
* [Run a script](#run-a-script)
* [Install a global tool](#install-a-global-tool)
* [Manage Rye](#manage-rye)

*See [commands](./commands.md) for more.*

## Getting started

### Installation

Rye is built in Rust. There is no binary distribution yet, it only works on Linux and macOS as of today:

```
$ cargo install --git https://github.com/mitsuhiko/rye rye
```

## Manage your Python toolchain

Use the `toolchain` command to use `rye` to manage your Python toolchain.

```zsh
$ rye toolchain list --include-downloadable
cpython@3.11.1
cpython@3.10.9
cpython@3.9.16
cpython@3.10.8 (downloadable)
cpython@3.10.7 (downloadable)
...
```

We'll download `cpython3.10.8` for a new project.

```zsh
$ rye toolchain fetch 3.10.8
Downloading cpython@3.10.8
success: Downloaded cpython@3.10.8
```

`rye` uses [python-build-standalone](https://github.com/indygreg/python-build-standalone). This means `rye` manages your project's Python environment with a standalone toolchain by downloading the builds for your system.

> 💡 Tip: You can register custom Python toolchains with `rye toolchain register`.

```zsh
$ rye toolchain register ~/Downloads/pypy3.9-v7.3.11-macos_arm64/bin/python
```

## Start a project

Use the `init` command to initialize a project.

```zsh
$ mkdir getting-started
$ cd getting-started
$ rye init
```

This command will bootstrap your directory as a Python project compatible with `rye`.

```zsh
$ tree -a .
.
├── .git
├── .gitignore
├── README.md
├── pyproject.toml
└── src
    └── getting_started
        └── __init__.py
```

A `pyproject.toml` is used to store metadata about your project as well as some `rye` configuration. Most of `rye`'s commands will require a `pyproject.toml` to work.


## Change your current Python version

The `pin` command is used to pin the current version of Python `rye` uses for the workspace. Use it to pin the newly downloaded `cpython@3.10.8` to the project.

```zsh
$ rye pin 3.10.8
```

`rye` can be used to manage your project, its `pyproject.toml`, and its environment.

```zsh
$ tree -a .
.
├── .git
├── .gitignore
├── .python-version
├── README.md
├── pyproject.toml
└── src
    └── getting_started
        └── __init__.py
```

## Add a dependency

Use the `add` command to add dependencies to your project.

```zsh
$ rye add "flask>=2.0"
$ rye add --dev black
```

> 💡 Tip: You can install dependencies from other sources as well.

```zsh
$ rye add package-name --git=ssh://git@git.example.com/MyProject
```

## Sync your Python environment

`rye` will sync your environment with the `sync` command.

```zsh
$ rye sync
```

When `rye sync` is run in a workspace all packages are installed together. This also means that they can inter-depend as they will all be installed editable by default.

```zsh
$ tree -a .
.
├── .git
├── .gitignore
├── .python-version
├── .venv
├── README.md
├── pyproject.toml
├── requirements-dev.lock
├── requirements.lock
└── src
    └── getting_started
        └── __init__.py
```

## Remove a dependency

Use the `remove` command to remove a dependency from the project.

```zsh
$ rye remove flask
$ rye sync
```

## Run a script

`rye run` can be used to invoke a binary from the virtualenv or a configured script. `rye` allows you to define basic scripts in the `pyproject.toml` in the `tool.rye.scripts` section:

```toml
[tool.rye.scripts]
serve = "python -m http.server 8000"
```

## Install a global tool

If you want tools to be installed into isolated virtualenvs (like `pipsi` and `pipx`), you can use `rye` (requires `~/.rye/shims` to be on the path):

```zsh
$ rye install pycowsay
$ pycowsay Wow

  ---
< Wow >
  ---
   \   ^__^
    \  (oo)\_______
       (__)\       )\/\
           ||----w |
           ||     ||
```

Alternatively, use `rye run <script>` to run a script (installed into `.venv/bin`) in the context of the virtual environment. This for instance can be used to run black:

```zsh
$ rye add --dev black
$ rye sync
$ rye run black .
```

To have multiple projects share the same virtualenv, it's possible to declare workspaces in the `pyproject.toml`:

```toml
[tool.rye.workspace]
members = ["foo-*"]
```

## Manage Rye

Update `rye` using the `self` command.

```zsh
$ rye self update
```

## Feedback

Submit an issue [here](https://github.com/mitsuhiko/rye/issues/new/choose).
