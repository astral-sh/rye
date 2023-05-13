# Installation

Rye is built in Rust. There is no binary distribution yet, it only works on
Linux and macOS as of today.  You need to have Rust and Cargo installed.  If you
don't have, you can use [rustup](https://rustup.rs/) to get them onto your machine.

Afterwards you can install `Rye` via `cargo`:

```bash
cargo install --git https://github.com/mitsuhiko/rye rye
```

## Add Shims to Path

Once `rye` is installed you should also add `~/.rye/shims` into your `PATH`.
This folder is a folder that contains "shims" which are executables that
Rye manages for you.  For instance any Python installation managed by Rye
will be available via a shim placed there.

On macOS or Linux you can accomplish this by adding it to your `.bashrc`, `.zshrc`
or similar.  This step is optional but required if you want to be able to
just type `python` into the shell to pick up the current virtualenv's Python
interpreter.  Likewise it's required if you want to take advantage of Rye's
global tool installation feature.

=== "Bash"

    ```bash
    echo 'export PATH="$HOME/.rye/shims:$PATH"' >> ~/.bashrc
    ```

=== "ZSH"

    ```bash
    echo 'export PATH="$HOME/.rye/shims:$PATH"' >> ~/.zshrc
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

If you don't want to use Rye any more, you can use `cargo` to uninstall it again:

```bash
cargo uninstall rye
```

Additionally you should delete the `.rye` folder from your home directory and
remove `~/.rye/shims` from the `Path` again.  Rye itself does not place any data
in other locations.  Note though that virtual environments created by rye will
no longer function after Rye was uninstalled.
