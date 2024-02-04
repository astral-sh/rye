<div align="center">
  <img src="docs/static/favicon.svg" width="100">
  <p><strong>Rye:</strong> a Hassle-Free Python Experience</p>
</div>

----
<div align="center">

[![Rye](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/mitsuhiko/rye/main/artwork/badge.json)](https://rye-up.com)
[![](https://dcbadge.vercel.app/api/server/drbkcdtSbg?style=flat)](https://discord.gg/drbkcdtSbg)

</div>

Rye is a project and package management solution for Python, created by
[Armin](https://github.com/mitsuhiko/).  It came out of his desire to create a
one-stop-shop for all Python needs. It installs and manages Python
installations, manages `pyproject.toml` files, installs and uninstalls
dependencies, manages virtualenvs behind the scenes. It supports monorepos and
global tool installations.

It is a wish of what Python was, an exploration of what is possible. It's far
from perfect, but always improving.  It also asks the question:
**[Should it exist?](https://github.com/mitsuhiko/rye/discussions/6)**

<div align="center">
  <a href="https://youtu.be/q99TYA7LnuA">
    <img src="https://img.youtube.com/vi/q99TYA7LnuA/hqdefault.jpg" alt="Watch the instruction" width="40%">
  </a>
  <p><em>Click on the thumbnail to watch a 16 minute introduction video</em></p>
</div>

Learn more:

* [Documentation](https://mitsuhiko.github.io/rye)
* [Issue Tracker](https://github.com/mitsuhiko/rye/issues)
* [Discussions](https://github.com/mitsuhiko/rye/discussions)
* [Discord](https://discord.gg/drbkcdtSbg)

## Usage

For installation instructions please refer to the [installation documentation](https://mitsuhiko.github.io/rye/guide/installation/).

To use rye for automatic management, you first need to create a new project using `rye init`:

```shell
$ rye init my_project && cd my_project
```

Once that's done, you can follow these steps to enjoy the benefits of automated management:

```shell
$ rye sync
```

If you want to choose a specific version of Python, you can use the `rye pin` command to specify the version you need (optionally):

``` shell
$ rye pin cpython@3.11
```

That's it! You can now easily achieve automatic management and switch between different versions of Python as needed.

The virtualenv that `rye` manages is placed in `.venv` next to your `pyproject.toml`.
You can activate and work with it as normal with one notable exception: the Python
installation in it does not contain `pip`.

Correctly installed, `rye` will automatically pick up the right Python without
manually activating the virtualenv.  That is enabled by having `~/.rye/shims` at
higher priority in your `PATH`.  If you operate outside of a rye managed
project, the regular Python is picked up automatically.

## Some of the things it does

It automatically installs and manages Python:

```
$ rye pin 3.11
$ rye run python
Python 3.11.1 (main, Jan 16 2023, 16:02:03) [Clang 15.0.7 ] on darwin
Type "help", "copyright", "credits" or "license" for more information.
>>>
```

**Note that does mean, that Rye will automatically download and
install an appropriate Python binary for you.** These Python binaries
are currently pulled from [the indygreg
python-build-standalone releases](https://github.com/indygreg/python-build-standalone/releases).

Install tools in isolation globally:

```
$ rye install maturin
```

Manage dependencies of a local `pyproject.toml` and update the virtualenv
automatically:

```
$ rye add flask
$ rye sync
```

## Decisions Made

To understand why things are the way they are:

- **Virtualenvs:** they are not pefect, but they are widespread.
  Because they have reasonable tooling support, the decision was made to
  leverage them over `__pypackages__`.

- **No Default Dependencies:** the virtualenvs when they come up are completely void
  of dependencies.  Not even `pip` or `setuptools` are installed into it.  Rye
  manages the virtualenv from outside the virtualenv.

- **No Core Non Standard Stuff:** Rye (with the exception of it's own `tool` section
  in the `pyproject.toml`) uses standardized keys. That means it uses regular
  requirements as you would expect. It also does not use a custom lock file
  format and uses [`pip-tools`](https://github.com/jazzband/pip-tools) behind the scenes.

- **No Pip:** Rye uses pip, but it does not expose it. It manage dependencies in
  `pyproject.toml` only.

- **No System Python:** To simplify the experience for users across operating systems,
  Rye manages Python installations for you, bypassing the system one by default in
  favor of [indygreg's Python builds](https://github.com/indygreg/python-build-standalone).
  Rye will automatically download and manage Python builds from there. No compiling,
  no divergence.

- **Project Local Shims:** Rye maintains a `python` shim that auto discovers the
  current `pyproject.toml` and automatically operates below it. Just add the
  shims to your shell and you can run `python` and it will automatically always
  operate in the right project.

## Python Distributions

Rye does not use system python installations. Instead it uses Gregory Szorc's standalone
Python builds: [python-build-standalone](https://github.com/indygreg/python-build-standalone).
This is done to create a unified experience of Python installations and to avoid
incompatibilities created by different Python distributions. Most importantly this also
means you never need to compile a Python any more, it just downloads prepared binaries.

## Global Tools

If you want tools to be installed into isolated virtualenvs (like pipsi and pipx), you
can use `rye` too (requires `~/.rye/shims` to be on the path):

```
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

To uninstall run `rye uninstall pycowsay` again.

## More

* [Discussion Forum](https://github.com/mitsuhiko/rye/discussions), to discuss the project
  on GitHub
* [Discord](https://discord.gg/drbkcdtSbg), for conversations with other developers in text form
* [Issue Tracker](https://github.com/mitsuhiko/rye/issues), if you run into bugs or have suggestions
* [Badges](https://rye-up.com/community/#badges), if you want to show that you use Rye
* License: MIT
