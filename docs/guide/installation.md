# Installation

Rye is built in Rust.  It can either be manually compiled and installed or it can
be installed from a binary distribution yet.  It has support for Linux, macOS and
Windows.

## Installing Rye

Rye is installed per-user and self manages itself.  It will install itself into
a folder in your home directory and mange itself there.

=== "Linux"

    To install run you can curl a command which will install the right binary for your
    operating system and CPU architecture and install it:

    ```bash
    curl https://raw.githubusercontent.com/mitsuhiko/rye/main/scripts/install.sh | bash
    ```

    Alternatively if you don't trust this approach, you can download the latest release
    binary.  On first run it will install itself.

    * [rye-x86_64-linux.gz](https://github.com/mitsuhiko/rye/releases/latest/download/rye-x86_64-linux.gz) for 64bit Intel computers

    ```bash
    gunzip rye-x86_64-linux.gz
    chmod +x ./rye-x86_64-linux
    ./rye-x86_64-linux
    ```

=== "macOS"

    To install run you can curl a command which will install the right binary for your
    operating system and CPU architecture and install it:

    ```bash
    curl https://raw.githubusercontent.com/mitsuhiko/rye/main/scripts/install.sh | bash
    ```

    Alternatively if you don't trust this approach, you can download the latest release
    binary.  On first run it will install itself.

    * [rye-aarch64-macos.gz](https://github.com/mitsuhiko/rye/releases/latest/download/rye-aarch64-macos.gz) for M1/M2 Macs
    * [rye-x86_64-macos.gz](https://github.com/mitsuhiko/rye/releases/latest/download/rye-x86_64-macos.gz) for Intel Macs

    ```bash
    gunzip rye-aarch64-macos.gz
    chmod +x ./rye-aarch64-macos
    ./rye-arch64-macos
    ```

=== "Windows"

    To install Rye on windows download the latest release and run the binary.  Upon
    first run it will install itself.

    * [rye-x86_64-windows.exe](https://github.com/mitsuhiko/rye/releases/latest/download/rye-x86_64-windows.exe) for 64bit Intel Windows
    * [rye-x86-windows.exe](https://github.com/mitsuhiko/rye/releases/latest/download/rye-x86-windows.exe) for 32bit Intel Windows

    !!!Note
    
        Rye does not yet use signed binaries which means that you will need to allow
        the execution of the downloaded executable.  If there is no obvious way to do so, click
        on "More info" on the error message that shows up and then on "Run anyway".

=== "Compile Yourself"

    You need to have Rust and Cargo installed.  If you don't have, you can use
    [rustup](https://rustup.rs/) to get them onto your machine.

    Afterwards you can install `Rye` via `cargo`:

    ```bash
    cargo install --git https://github.com/mitsuhiko/rye rye
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