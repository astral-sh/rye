=== "Linux"

    To install run you can curl a command which will install the right binary for your
    operating system and CPU architecture and install it:

    ```bash
    curl -sSf https://rye-up.com/get | bash
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
    curl -sSf https://rye-up.com/get | bash
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
