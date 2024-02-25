<div align="center">
  <img src="docs/static/favicon.svg" width="100">
  <p><strong>Rye:</strong> a Hassle-Free Python Experience</p>
</div>

----
<div align="center">

[![Rye](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/rye/main/artwork/badge.json)](https://rye-up.com)
[![Rye Version](https://img.shields.io/github/v/release/astral-sh/rye?logo=github&logoColor=D7FF64&label=Version&color=261230)](https://github.com/astral-sh/rye/releases/)
[![GitHub](https://img.shields.io/github/license/astral-sh/rye?logo=github&logoColor=D7FF64&color=261230)](https://github.com/astral-sh/rye/blob/main/LICENSE)
[![GitHub Actions](https://img.shields.io/github/actions/workflow/status/astral-sh/rye/release.yml?logo=github&logoColor=D7FF64&color=261230)](https://github.com/astral-sh/rye/actions/workflows/release.yml)
[![Astral Discord](https://img.shields.io/discord/1039017663004942429?logo=discord&logoColor=D7FF64&label=Astral%20Discord&color=261230)](https://discord.gg/astral-sh)
[![Rye Discord](https://img.shields.io/discord/1108809133131563092?logo=discord&logoColor=D7FF64&label=Rye%20Discord&color=261230)](https://discord.gg/drbkcdtSbg)

</div>

Rye is a comprehensive project and package management solution for Python.
Born from [its creator's](https://github.com/mitsuhiko) desire to establish a
one-stop-shop for all Python users, Rye provides a unified experience to install and manage Python
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
* **Locking and Dependency Installation:** is today implemented by using [uv](https://github.com/astral-sh/uv) with a fallback to [unearth](https://pypi.org/project/unearth/) and [pip-tools](https://github.com/jazzband/pip-tools/).
* **Workspace support:** Rye lets you work with complex projects consisting
  of multiple libraries.

## Installation

The installation takes just a minute:

* **Linux and macOS:**

    ```
    curl -sSf https://rye-up.com/get | bash
    ```

* **Windows:**

    Download and run the installer ([64bit Intel](https://github.com/astral-sh/rye/releases/latest/download/rye-x86_64-windows.exe) or [32bit Intel](https://github.com/astral-sh/rye/releases/latest/download/rye-x86-windows.exe)).

For more details and other options, refer to the [installation instructions](https://rye-up.com/guide/installation/).

## Learn More

Did we spark your interest?

* [Visit the Website](https://rye-up.com/)
* [Read the Documentation](https://rye-up.com/guide/)
* [Report Problems in the Issue Tracker](https://github.com/astral-sh/rye/issues)

## More

* [Discussion Forum](https://github.com/astral-sh/rye/discussions), to discuss the project
  on GitHub
* [Discord](https://discord.gg/drbkcdtSbg), for conversations with other developers in text form
* [Issue Tracker](https://github.com/astral-sh/rye/issues), if you run into bugs or have suggestions
* [Badges](https://rye-up.com/community/#badges), if you want to show that you use Rye
* License: MIT

## Astral

Rye now is a project maintained by the [Astral][astral] team:
* [Rye grows with `uv`](https://lucumr.pocoo.org/2024/2/15/rye-grows-with-uv/)
* [`uv`: Python packaging in Rust](https://astral.sh/blog/uv)


<div align="center">
  <a target="_blank" href="https://astral.sh" style="background:none">
    <img src="https://raw.githubusercontent.com/astral-sh/uv/main/assets/svg/Astral.svg" alt="Made by Astral">
  </a>
</div>

[astral]: https://astral.sh
