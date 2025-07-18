!!! note

    Rye is no longer developed as of February 2025. We encourage all users to use
    [uv](https://docs.astral.sh/uv/), the [successor project](https://lucumr.pocoo.org/2024/8/21/harvest-season/)
    from the same maintainers, which is actively maintained and much more widely used.

    For new users, please [install uv](https://docs.astral.sh/uv/getting-started/installation/) instead.

    For current Rye users, please see the [uv migration guide](uv.md). While these installation
    instructions will continue to work for the foreseeable future, no further updates are planned,
    including security updates.

# Installation

Rye is built in Rust.  It can either be manually compiled and installed or it can
be installed from a binary distribution.  It has support for Linux, macOS and
Windows.

## Installing Rye

Rye is installed per-user and self manages itself.  It will install itself into
a folder in your home directory and manage itself there.

{% include-markdown "../.includes/quick-install.md" %}

Rye will automatically download suitable Python toolchains as needed.  For more
information about this [read about toolchains](toolchains/index.md).  To install
a specific version download a binary directly
[from GitHub](https://github.com/astral-sh/rye/releases).

## Customized Installation

On some platforms there is some limited support for customizing the installation
experience.  This for instance can be necessary on certain Linux environments such
as Alpine where the Rye provided Python interpreter is not supported.

=== "Linux"

    {% include-markdown "../.includes/curl-to-bash-options.md" %}

=== "macOS"

    {% include-markdown "../.includes/curl-to-bash-options.md" %}

=== "Windows"

    The Windows installer has limited support for customizations via environment
    variables.  To set these you need to run the installer from `cmd.exe`.

    {% include-markdown "../.includes/installer-options.md" %}

    This for instance installs Rye with a specific toolchain:

    ```batch
    set RYE_TOOLCHAIN=%USERPROFILE%\AppData\Local\Programs\Python\Python310\python.exe
    rye-x86_64-windows.exe
    ```

## Add Shims to Path

Once `rye` is installed you need to add the `shims` folder into your `PATH`.
This folder is a folder that contains "shims" which are executables that
Rye manages for you as well as the `rye` executable itself.  For instance any
Python installation managed by Rye will be available via a shim placed there.

On macOS or Linux you can accomplish this by adding it to your `.profile` file
or similar.  This step is technically optional but required if you want to be able to
just type `python` or `rye` into the shell to pick up the current virtualenv's Python
interpreter.  The installer will offer to do this for you automatically.  If you
opt-out, or you run a custom shell you will need to do this manually.

=== "Bash"

    Rye ships an `env` file which should be sourced to update `PATH` automatically.

    ```bash
    echo 'source "$HOME/.rye/env"' >> ~/.profile
    ```

    In some setups `.profile` is not sourced, in which case you can add it to your
    `.bashrc`:

    ```bash
    echo 'source "$HOME/.rye/env"' >> ~/.bashrc
    ```

=== "ZSH"

    Rye ships an `env` file which should be sourced to update `PATH` automatically.

    ```bash
    echo 'source "$HOME/.rye/env"' >> ~/.zprofile
    ```

    In some setups `.zprofile` is not sourced, in which case you can add it to your
    `.zshrc`:

    ```bash
    echo 'source "$HOME/.rye/env"' >> ~/.zshrc
    ```

=== "Fish"

    Since fish does not support `env` files, you need to add
    the shims directly.  This can be accomplished by running this
    command once:

    ```bash
    set -Ua fish_user_paths "$HOME/.rye/shims"
    ```

=== "Nushell"

    Since nushell does not support `env` files, you need to add
    the shims directly.  This can be accomplished by adding this to your
    `env.nu` file:

    ```shell
    $env.PATH = ($env.PATH | split row (char esep) | append "~/.rye/shims")
    ```

=== "Unix Shells"

    Rye ships an `env` file which should be sourced to update `PATH` automatically.

    ```bash
    echo '. "$HOME/.rye/env"' >> ~/.profile
    ```

=== "Windows"

    The windows installer normally will automatically register the rye path in the
    `PATH` environment variable.  If this does not work you will need to manually
    perform the following steps:

    1. Press ++windows+r++, enter `sysdm.cpl` and hit ++enter++.
    2. In the "System Properties" dialog, click the "Advanced" tab.
    3. Click on "Environment Variables".
    4. In the top list, double click on the `Path` variable.
    5. In the "Edit environment variable" dialog click on "New".
    6. Enter `%USERPROFILE%\.rye\shims` and hit ++enter++.
    7. Click repeatedly on "Move Up" until the newly added item is at the top.
    8. Click on "OK" and close the dialog.

    Note that you might need to restart your login session for this to take effect.

There is a quite a bit to shims and their behavior.  Make sure to [read up on shims](shims.md)
to learn more.

## Shell Completion

Rye supports generating completion scripts for Bash, Zsh, Fish, Powershell and Nushell. Here are some common locations for each shell:

=== "Bash"

    ```bash
    mkdir -p ~/.local/share/bash-completion/completions
    rye self completion > ~/.local/share/bash-completion/completions/rye.bash
    ```

=== "Zsh"

    ```bash
    # Make sure ~/.zfunc is added to fpath, before compinit.
    rye self completion -s zsh > ~/.zfunc/_rye
    ```

    Oh-My-Zsh:

    ```bash
    mkdir $ZSH_CUSTOM/plugins/rye
    rye self completion -s zsh > $ZSH_CUSTOM/plugins/rye/_rye
    ```

    Then make sure rye plugin is enabled in ~/.zshrc

=== "Fish"

    ```bash
    rye self completion -s fish > ~/.config/fish/completions/rye.fish
    ```

=== "Powershell"

    ```ps1
    # Create a directory to store completion scripts
    mkdir $PROFILE\..\Completions
    echo @'
    Get-ChildItem "$PROFILE\..\Completions\" | ForEach-Object {
        . $_.FullName
    }
    '@ | Out-File -Append -Encoding utf8 $PROFILE
    # Generate script
    Set-ExecutionPolicy Unrestricted -Scope CurrentUser
    rye self completion -s powershell | Out-File -Encoding utf8 $PROFILE\..\Completions\rye_completion.ps1
    ```

=== "NuShell"

    ```nushell
    rye self completion -s nushell | save --append $nu.env-path
    ```

## Updating Rye

To update rye to the latest version you can use `rye` itself:

```
rye self update
```

## Uninstalling

If you don't want to use Rye any more, you can ask it to uninstall it again:

```bash
rye self uninstall
```

Additionally you should delete the remaining `.rye` folder from your home directory and
remove `.rye/shims` from the `PATH` again (usually by removing the code that sources
the `env` file from the installation step).  Rye itself does not place any data
in other locations.  Note though that virtual environments created by rye will
no longer function after Rye was uninstalled.

## Preventing Auto Installation

Rye when launched will normally perform an auto installation.  This can be annoying
in certain development situations.  This can be prevented by exporting the
`RYE_NO_AUTO_INSTALL` environment variable.  It needs to be set to `1` to disable
the feature.

=== "Linux"

    ```bash
    export RYE_NO_AUTO_INSTALL=1
    ```

=== "macOS"

    ```bash
    export RYE_NO_AUTO_INSTALL=1
    ```

=== "Windows"

    ```bash
    set RYE_NO_AUTO_INSTALL=1
    ```
