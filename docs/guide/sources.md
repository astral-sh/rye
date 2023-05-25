# Dependency Sources

+++ 0.2.0

Normally Rye loads packages from PyPI only.  However it is possible to instruct it to
load packages from other indexes as well.

## Adding a Source

An index can be added to a project or workspace (via `pyproject.toml`) or into the
[global config](config.md#config-file).  Rye will always consult both files where the
`pyproject.toml` file wins over the global config.

Each source needs to have a unique name.  The default source is always called `default`
and out of the box points to PyPI.

=== "Global Source"

    Add this to `~/.rye/config.toml`:

    ```toml
    [[sources]]
    name = "company-internal"
    url = "https://company.internal/simple/"
    ```

=== "Project Source"

    Add this to `pyproject.toml`:

    ```toml
    [[tool.rye.sources]]
    name = "company-internal"
    url = "https://company.internal/simple/"
    ```

### Index Types

Rye supports different types of sources and also allows overriding the `default`
PyPI index.  If you give another source the name `default`, PyPI will no longer be
used for resolution.

=== "Regular Index"

    ```toml
    [[sources]]
    name = "company-internal"
    url = "https://company.internal/simple/"
    type = "index"  # this is implied
    ```

=== "Find Links"

    ```toml
    [[sources]]
    name = "company-internal"
    url = "https://company.internal/"
    type = "find-links"
    ```

=== "Default Index"

    ```toml
    [[sources]]
    name = "default"
    url = "https://company.internal/simple/"
    ```

    !!! Warning

        Please take note that the default index cannot be of type `find-links`.

## Source Types

The two sources types (`index` vs `find-links`) are determined by the underlying pip
infrastructure:

### `index`

This is a [PEP 503](https://www.python.org/dev/peps/pep-0503/) type index as provided
by tools such as PyPI or [devpi](https://github.com/devpi/devpi).  It corresponds to
the arguments `--index-url` or `--extra-index-url` in pip.

### `find-links`

This is a source that can be of a variety of types and has to point to a file path
or hosted HTML page linking to packages.  It corresponds to the `--find-links`
argument.  The format of the HTML page is somewhat underspecified but generally
all HTML links pointing to `.tar.gz` or `.whl` files are considered.

## Index Authentication

HTTP basic auth is supported for index authentication.  It can be supplied in two
ways.  `username` and `password` can be directly embedded in the config, or they
can be supplied with environment variables.

=== "Configured Credentials"

    ```toml
    [[sources]]
    name = "company-internal"
    url = "https://company.internal/simple/"
    username = "username"
    password = "super secret"
    ```

=== "Environment Variables"

    ```toml
    [[sources]]
    name = "company-internal"
    url = "https://${INDEX_USERNAME}:${INDEX_PASSWORD}@company.internal/simple/"
    ```