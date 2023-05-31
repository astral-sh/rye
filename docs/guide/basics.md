# Basics

To use Rye you need to have a `pyproject.toml` based Python project.  For this guide you can
create one with `rye init` which will create a new folder with a new project inside:

```shell
rye init my-project
cd my-project
```

The following structure will be created:

```
.
├── .git
├── .gitignore
├── .python-version
├── README.md
├── pyproject.toml
└── src
    └── my_project
        └── __init__.py
``` 

A `pyproject.toml` is used to store metadata about your project as well as some Rye
configuration.  Most of Rye's commands will require a `pyproject.toml` to work.  Note
that Rye today does not support `setup.py` based projects.  Note that when Rye initializes
a project it also writes a `.python-version` file.  This file contains the version number
of the Python version that should be used for this project.  It can be changed by
running `rye pin`.  For instance to tell Rye to use Python 3.10:

```
$ rye pin 3.10
```

## First Sync

Once that is done, you can use `rye sync` to get the first synchronization.  After that,
Rye will have created a virtualenv in `.venv` and written lockfiles into `requirements.lock`
and `requirements-dev.lock`.

```shell
rye sync
```

The virtualenv that Rye manages is placed in `.venv` next to your `pyproject.toml`.
The first time you run this you will notice that Rye automatically downloaded and
installed a compatible CPython interpreter for you.  If you have already another
Python installation on your system it will not be used!  For more information about
this behavior [read about toolchains](toolchains/index.md).

You can activate and work with it as normal with one notable exception: the Python
installation in it does not contain `pip`.  If you have correctly installed Rye
with the shims enabled, after the sync you can run `python` and you will automatically
be operating in that virtualenv, even if it's not enabled.  You can validate this
by printing out `sys.prefix`:

```
python -c "import sys; print(sys.prefix)"
```

It will print out the full path to the managed virtualenv.

## Adding Dependencies

Use the `add` command to add dependencies to your project.

```zsh
rye add "flask>=2.0"
```

Note that after `add` you need to run `sync` again to actually install it.  If you
want to add packages from custom indexes, you have to [configure the source](sources.md)
first.

## Remove a Dependency

Use the `remove` command to remove a dependency from the project again.

```zsh
rye remove flask
```

## Working with the Project

To run executables in the context of the virtualenv you can use the `run` command.  For
instance if you want to use `black` you can add and run it like this:

```
rye add black
rye sync
rye run black
```

To activate the virtualenv, use the standard methods:

=== "Unix"

    ```zsh
    . .venv/bin/activate
    ```

=== "Windows"

    ```bat
    .venv\Scripts\activate
    ```

To deactivate it again run `deactivate`:

```
deactivate
```