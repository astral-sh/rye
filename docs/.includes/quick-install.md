=== "Linux"

    To install you can run a curl command which will install the right binary for your
    operating system and CPU architecture and install it:

    ```bash
    curl -sSf https://rye.astral.sh/get | bash
    ```

    Alternatively if you don't trust this approach, you can download the latest release
    binary.  On first run it will install itself.

    * [rye-x86_64-linux.gz](https://github.com/astral-sh/rye/releases/latest/download/rye-x86_64-linux.gz) Intel/AMD (x86-64).
    * [rye-aarch64-linux.gz](https://github.com/astral-sh/rye/releases/latest/download/rye-aarch64-linux.gz) for ARM64.

    ```bash
    gunzip rye-x86_64-linux.gz
    chmod +x ./rye-x86_64-linux
    ./rye-x86_64-linux
    ```

=== "macOS"

    To install you can run a curl command which will install the right binary for your
    operating system and CPU architecture and install it:

    ```bash
    curl -sSf https://rye.astral.sh/get | bash
    ```

    Alternatively if you don't trust this approach, you can download the latest release
    binary.  On first run it will install itself.

    * [rye-aarch64-macos.gz](https://github.com/astral-sh/rye/releases/latest/download/rye-aarch64-macos.gz) for Apple Silicon (M1/M2/M3) (ARM64).
    * [rye-x86_64-macos.gz](https://github.com/astral-sh/rye/releases/latest/download/rye-x86_64-macos.gz) for Intel processors (x86-64).

    ```bash
    gunzip rye-aarch64-macos.gz
    chmod +x ./rye-aarch64-macos
    ./rye-aarch64-macos
    ```

=== "Windows"

    To install Rye on windows download the latest release and run the binary.  Upon
    first run it will install itself.  Please note that it's strongly recommended
    to have "Developer Mode" activated when using Rye and before starting the
    installation.  [Learn more](../guide/faq.md).

    * [rye-x86_64-windows.exe](https://github.com/astral-sh/rye/releases/latest/download/rye-x86_64-windows.exe) for 64-bit (x86-64).
    * [rye-x86-windows.exe](https://github.com/astral-sh/rye/releases/latest/download/rye-x86-windows.exe) for 32-bit (x86).

    !!!Note
    
        Rye does not yet use signed binaries which means that you will need to allow
        the execution of the downloaded executable.  If there is no obvious way to do so, click
        on "More info" on the error message that shows up and then on "Run anyway".

        Additionally sometimes a Trojan warning about "Bearfoos" is shown.  This is a false
        positive.  For more information see the discussion [Windows Bearfoos
        virus associated with rye](https://github.com/astral-sh/rye/issues/468).

=== "Compile Yourself"

    You need to have Rust and Cargo installed.  If you don't have, you can use
    [rustup](https://rustup.rs/) to get them onto your machine.

    Afterwards you can install `Rye` via `cargo`:

    ```bash
    cargo install --git https://github.com/astral-sh/rye rye
    ```
