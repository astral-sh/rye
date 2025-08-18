# Migrating to uv

Rye is no longer developed. We recommend that Rye users migrate to [uv](https://docs.astral.sh/uv/), the [successor project](https://lucumr.pocoo.org/2024/8/21/harvest-season/) from the same maintainers. uv is actively maintained and much more widely used, and it supports almost all of Rye's features (as well as several features not available in Rye).

Rye and uv have similar philosophies, CLIs, and user experience. Both are project management tools that follow Python standards wherever possible, so migrating your workflows should be easy. Start with [uv's getting started guide](https://docs.astral.sh/uv/getting-started/).

This document has guidance on how to migrate specific Rye features to uv, targeted at uv 0.8 / July 2025. If you're on an older version of uv, please upgrade. (If you are reading this page in the future, check the linked issues below to see if even more features have been implemented in uv!)

## Migrating existing projects

If you don't have `[tool.rye]` sections in your `pyproject.toml` file, your project should work out of the box in uv with no further changes.

If you have `[tool.rye]` sections in your `pyproject.toml` file, you will need to convert them to `[tool.uv]` sections. Most of the time, all you need to do is rename the sections.

A few configuration keys are slightly different but easy to migrate:

* `tool.rye.universal` and `tool.rye.generate-hashes` are under the `tool.uv.pip` namespace, i.e., `tool.uv.pip.universal` and `tool.uv.pip.generate-hashes`.
* `tool.rye.lock-with-sources` corresponds to `tool.uv.no-sources` but the Boolean value of the setting is inverted, i.e., convert `lock-with-sources = false` to `no-sources = true`.
* `tool.rye.virtual` corresponds to `tool.uv.package` but with the sense inverted, i.e., convert `virtual = true` to `package = false`.
* `tool.rye.sources` loosely corresponds to `tool.uv.indexes`. See below for syntax differences.
* `tool.rye.dev-dependencies` should be converted to the standardized [`dependency-groups.dev`](https://docs.astral.sh/uv/concepts/projects/dependencies/#dependency-groups) key (`tool.uv.dev-dependencies` works too but is deprecated).

See also the [uv configuration reference](https://docs.astral.sh/uv/reference/settings/).

### Scripts and tasks: `[tool.rye.scripts]`

As of July 2025, uv does not yet have a task runner. Please see the feature request in [astral-sh/uv#5903](https://github.com/astral-sh/uv/issues/5903), which also has links to some third-party alternatives that are usable today via `uv run`.

(Note that this is not related to executable scripts in the standard `[project.scripts]` section, which is fully supported in uv.)

### Indexes and repos: `[[tool.rye.sources]]`

uv's support for configuring package indexes is, in general, more flexible than Rye's. Indexes are listed in `[[tool.uv.indexes]]` (which corresponds to `[[tool.rye.sources]]`), and you can specify that you want specific packages in your requirements to come from a particular index using `[[tool.uv.sources]]`. To override the default index, use `default = true` instead of `name = "default"`. For a "find-links" index, use `format = "flat"` instead of `type = "find-links"`.

See the [uv documentation on package indexes](https://docs.astral.sh/uv/concepts/indexes/) for more details.

Rye expands environment variables in the URLs of indexes. uv currently does not support this exact feature (see [astral-sh/uv#5734](https://github.com/astral-sh/uv/issues/5734)), but for most use cases there are alternatives:

* If you are filling in a username and password, [uv supports](https://docs.astral.sh/uv/concepts/indexes/#authentication) setting them via environment variables or loading them from an external keyring provider.
* If you are filling in the actual URL of an index, you can instead specify the full URL of an index on the command line with the [`UV_INDEX`](https://docs.astral.sh/uv/reference/environment/#uv_index) or [`UV_DEFAULT_INDEX`](https://docs.astral.sh/uv/reference/environment/#uv_default_index) environment variables.
* If you are using custom indexes for PyTorch, consider the [`torch-backend = auto`](https://docs.astral.sh/uv/reference/settings/#pip_torch-backend) setting.

You can also configure indexes in [global configuration](https://docs.astral.sh/uv/concepts/configuration-files/) to avoid placing them in your project-specific `pyproject.toml` file.

## Migrating command-line usage and workflows

While you will likely find the `uv` CLI tool familiar if you're used to `rye`, they have their own interfaces. Please read the [getting started guide](https://docs.astral.sh/uv/getting-started/features/) and the [`uv` CLI reference](https://docs.astral.sh/uv/reference/cli/).

### Python installations and shims

Both uv and Rye support installing the same managed Python releases from [python-build-standalone](https://github.com/astral-sh/python-build-standalone) (now under Astral stewardship), so you should find Python behavior to be highly compatible.

Rye supports a [global Python shim](https://rye.astral.sh/guide/shims/) that sets up the `python` and `python3` commands to run a managed version of Python. While this is highly convenient, it does risk breaking existing software on your computer that expects the OS version of Python.

As of uv 0.8.0, `uv python install` will install a `python3.x` command for the latest stable managed Python. If you would like to install `python` and `python3` commands too. you can use `uv python install --default`. Note that this is _not_ a shim, just a direct link to the Python installation; it does not pick up dependencies from the current project. This behavior may change in the future; [astral-sh/uv#6265](https://github.com/astral-sh/uv/issues/6265) tracks uv adding a Python shim.

In general, you can use `uv run python` to get a Python shell in the current project, and `uvx python` to get an isolated Python. Both commands support `--with` to add dependencies. See the documentation for [`uv run`](https://docs.astral.sh/uv/reference/cli/#uv-run).

### `rye lint`, `rye fmt`, and `rye test`

Rye has short commands that run other programs. `rye lint` (aka `rye check`) and `rye fmt` (aka `rye format`) both run [Ruff](https://docs.astral.sh/ruff/), and `rye test` runs [pytest](https://docs.pytest.org/en/stable/). uv currently does not have specific aliases for these. (See also the section above about scripts and tasks.)

You can add these to your projects as dev dependencies with `uv add --dev ruff` and `uv add --dev pytest`, which will let you run `uv run ruff check`, `uv run ruff format`, and `uv run pytest`. The versions of Ruff and pytest will be tracked in your `pyproject.toml` for reproducibility.

Alternatively, you can use `uvx ruff` to run it without installing it to your project. (This will not work for pytest, which needs to run your project's code; you need to add it as a dev dependency, just as `rye test` requires.)

### Project creation

`rye init --script` creates a project with a script runnable with `uv run`. The corresponding uv command is `uv init --package`, which adds an executable script by default. `uv init --script` does something else: it creates a [single-file executable script that supports dependencies](https://docs.astral.sh/uv/guides/scripts/) with a PEP 723 header, a feature Rye does not support.

### Lockfiles, Docker

Rye generates a lockfile named `requirements.lock` with a syntax compatible with `pip` and other tools that handle `requirements.txt` files.

uv uses its own lock file format (`uv.lock`). If you need a `requirements.txt`-style lock file for use with other tools, you can generate one with [`uv export`](https://docs.astral.sh/uv/concepts/projects/sync/#exporting-the-lockfile).

However, we strongly recommend using uv to manage your environment installation in production, too! In particular, Rye's Docker documentation suggests using `uv pip install` or `pip install` to install from the `requirements.lock` file. Instead of trying to adapt this workflow, take a look at [uv's Docker integration guide](https://docs.astral.sh/uv/guides/integration/docker/), which recommends using uv directly in your Docker image, with `uv sync` in your build step and `uv run` in your `CMD`.

## Feedback

If you have issues migrating a Rye project to uv, please feel free to [ask for help in one of uv's support forums](https://docs.astral.sh/uv/getting-started/help/) such as GitHub or Discord.

Thank you for using Rye! We hope you enjoy uv.
