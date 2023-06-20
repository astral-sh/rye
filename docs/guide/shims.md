# Shims

After installation Rye places two shims on your `PATH`: `python` and `python3`.  These
shims have specific behavior that changes depending on if they are used within a Rye
managed project or outside.

Inside a Rye managed project the resolve to the Python interpreter of the virtualenv.
This means that even if you do not enable the virtualenv, you can just run `python`
in a shell, and it will automatically operate in the right environment.

Outside a Rye managed project it typically resolves to your system Python, though you
can also opt to have it resolve to a Rye managed Python installation for you.  This is
done so that it's not disruptive to your existing workflows which might depend on the
System python installation.

## Global Shims

+++ 0.9.0

To enable global shims, you need to enable the `global-python` flag in
the [`config.toml`](config.md) file:

```bash
rye config --set-bool behavior.global-python=true
```

Afterwards if you run `python` outside of a Rye managed project it will
spawn a Python interpreter that is shipped with Rye.  It will honor the
closest `.python-version` file for you.  Additionally you can also
explicitly request a specific Python version by adding `+VERSION` after
the `python` command.  For instance this runs a script with Python 3.8:

```bash
python +3.8 my-script.py
```

!!! Note

    Selecting a specific Python version this way only works outside of
    Rye managed projects.  Within Rye managed projects, the version needs
    to be explicitly selected via `.python-version` or with the
    `requires-python` key in `pyproject.toml`.
