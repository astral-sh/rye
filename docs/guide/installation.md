# Installation

Rye is built in Rust.  It can either be manually compiled and installed or it can
be installed from a binary distribution yet.  It has support for Linux, macOS and
Windows.

## Installing Rye

Rye is installed per-user and self manages itself.  It will install itself into
a folder in your home directory and mange itself there.

{% include-markdown "../.includes/quick-install.md" %}

Rye will automatically download suitable Python toolchains as needed.  For more
information about this [read about toolchains](toolchains/index.md).  To install
a specific version download a binary directly
[from GitHub](https://github.com/mitsuhiko/rye/releases).

## Customized Installation

On some platforms there is some limited support for customizing the installation
experience.

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

On macOS or Linux you can accomplish this by adding it to your `.bashrc`, `.zshrc`
or similar.  This step is technically optional but required if you want to be able to
just type `python` or `rye` into the shell to pick up the current virtualenv's Python
interpreter.

=== "Bash"

    Rye ships an `env` file which should be sourced to update `PATH` automatically.

    ```bash
    echo 'source "$HOME/.rye/env"' >> ~/.bashrc
    ```

=== "ZSH"

    Rye ships an `env` file which should be sourced to update `PATH` automatically.

    ```bash
    echo 'source "$HOME/.rye/env"' >> ~/.zshrc
    ```

=== "Unix Shells"

    Rye ships an `env` file which should be sourced to update `PATH` automatically.

    ```bash
    echo '. "$HOME/.rye/env"' >> ~/.profile
    ```

=== "Windows"

    To modify the Windows PATH environment variable
    
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
remove `.rye/shims` from the `PATH` again.  Rye itself does not place any data
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
