<div align="center">
  <img src="docs/static/favicon.svg" width="100">
  <p><strong>Rye:</strong> a Hassle-Free Python Experience</p>
</div>

----
<div align="center">

[![Rye](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/mitsuhiko/rye/main/artwork/badge.json)](https://rye-up.com)
[![](https://dcbadge.vercel.app/api/server/drbkcdtSbg?style=flat)](https://discord.gg/drbkcdtSbg)

</div>

Rye is a comprehensive project and package management solution for Python.
Born from [its creator's](https://github.com/mitsuhiko) desire to establish a
one-stop-shop for all Python users, Rye provides a unified experience to install and manages Python
installations, `pyproject.toml` based projects, dependencies and virtualenvs
seamlessly.  It's designed to accommodate complex projects, monorepos and to
facilitate global tool installations.  Curious? [Watch an introduction](https://youtu.be/q99TYA7LnuA).

A hassle-free experience for Python developers at every level.

<div align="center">
  <a href="https://youtu.be/q99TYA7LnuA">
    <img src="https://img.youtube.com/vi/q99TYA7LnuA/hqdefault.jpg" alt="Watch the instruction" width="40%">
  </a>
  <p><em>Click on the thumbnail to watch a 16 minute introduction video</em></p>
</div>

## In The Box

Rye picks and ships the right tools so you can get started in minutes:

* **Bootstraps Python:** it provides an automated way to get access to the amazing [Indygreg Python Builds](https://github.com/indygreg/python-build-standalone/) as well as the PyPy binary distributions.
* **Linting and Formatting:** it bundles [ruff](https://github.com/astral-sh/ruff) and makes it available with `rye lint` and `rye fmt`.
* **Managing Virtualenvs:** it uses the well established virtualenv library under the hood.
* **Building Wheels:** it delegates that work largely to [build](https://pypi.org/project/build/).
* **Publishing:** its publish command uses [twine](https://pypi.org/project/twine/) to accomplish this task.
* **Locking and Dependency Installation:** is today implemented by using [unearth](https://pypi.org/project/unearth/) and [pip-tools](https://github.com/jazzband/pip-tools/).
* **Workspace support:** Rye lets you work with complex projects consisting
  of multiple libraries.

## Installation

The installation takes just a minute:

* **Linux and macOS:**

    ```
    curl -sSf https://rye-up.com/get | bash
    ```

* **Windows:**

    Download and run the installer ([64bit Intel](https://github.com/mitsuhiko/rye/releases/latest/download/rye-x86_64-windows.exe) or [32bit Intel](https://github.com/mitsuhiko/rye/releases/latest/download/rye-x86-windows.exe)).

For more details and other options, refer to the [installation instructions](https://rye-up.com/guide/installation/).

## Learn More

Did I spark your interest?

* [Visit the Website](https://rye-up.com/)
* [Read the Documentation](https://rye-up.com/guide/)
* [Report Problems in the Issue Tracker](https://github.com/mitsuhiko/rye/issues)

## More

* [Discussion Forum](https://github.com/mitsuhiko/rye/discussions), to discuss the project
  on GitHub
* [Discord](https://discord.gg/drbkcdtSbg), for conversations with other developers in text form
* [Issue Tracker](https://github.com/mitsuhiko/rye/issues), if you run into bugs or have suggestions
* [Badges](https://rye-up.com/community/#badges), if you want to show that you use Rye
* License: MIT
