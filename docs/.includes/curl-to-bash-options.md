The install script that is piped to `bash` can be customized with some environment
variables:

`RYE_VERSION`

:   Defaults to `latest`.  Can be set to an explicit version to install a specific one.

`RYE_INSTALL_OPTION`

:   Can optionally be set to `"--yes"` to skip all prompts.

{% include-markdown "../.includes/installer-options.md" %}

This for instance installs a specific version of Rye without asking questions:

```bash
curl -sSf https://rye-up.com/get | RYE_VERSION="0.4.0" RYE_INSTALL_OPTION="--yes" bash
```