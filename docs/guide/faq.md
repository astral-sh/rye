# FAQ

This section should cover some commonly asked questions.  If you do not find an answer
here, consider reaching out to the [community](../community.md).

## How Do I Install PyTorch?

PyTorch requires setting up manual [sources](../sources) as it's not installed via
PyPI.  These sources can be set up in [`pyproject.toml`](../pyproject/) for a
simple project or globally in the [config](../config/).

* **Option 1:** `pyproject.toml`

    ```toml
    [[tool.rye.sources]]
    name = "pytorch"
    url = "https://download.pytorch.org/whl/cpu"
    ```

* **Option 2:** `~/.rye/config.toml`

    ```toml
    [[sources]]
    name = "pytorch"
    url = "https://download.pytorch.org/whl/cpu"
    ```

Afterwards you can add pytorch as you would expect:

```
rye add torch torchvision torchaudio
```

## Windows Developer Mode

Rye does not require symlinks but it works significantly better with them.  On Windows
support for symlinks is restricted to privileged accounts.  The reason for this is that
Symlinks were a late addition to Windows and some applications are not developed with
them in mind which can cause misbehavior or in the worst case security issues in those
applications.  Symlinks support however is enabled when the "developer mode" is activated
on modern Windows versions.  

Enabling "developer mode" has changed in later version of Windows. For older versions:

1. Press ++windows+i++ to open the settings
2. In the settings dialog click on "Privacy & security"
3. In the "Security" section click on "For developers"
4. Enable the toggle "Developer Mode"
5. In the "Use developer features" dialog confirm by clicking "Yes".

In more modern versions:

1. Press ++windows+i++ to open the settings
2. In the settings dialog click on "System"
3. In the "System" section click on "For developers"
4. Enable the toggle "Developer Mode"
5. In the "Use developer features" dialog confirm by clicking "Yes".

??? question "What happens if I don't enable it?"

    Enabling symlinks is not strictly required as Rye automatically falls back to
    hardlinks and junction points.  However not having symlinks enabled will ultimately
    result in a worse user experience for the following reasons:

    * Custom toolchain registration uses proxy files rather than actual symlinks which
      means that the executables in the `.rye\py` path are non executable.
    * All shims will be installed as hardlinks.  This can cause issues when upgrading
      Rye while Python is in use.  These hardlinks will also continue to point to older
      Rye executables creating more hard drive usage.
    * Virtualenvs will be created with copies rather than symlinks.
    * Junction points are used where symlinks to directories are otherwise used.  Some
      tools might accidentally not detect junction points which can cause deletion of
      virtualenvs to accidentally also delete or destroy the toolchain behind it.

## Missing Shared Libraries on Linux

The Python builds that Rye uses require a Linux installation compatible to the
Linux Standard Base Core Specification (LSB).  Unfortunately not all Linux
distributions are strictly adhering to that specification out of the box.  In
particularly the library `libcrypt.so.1` is commonly not installed on certain
Linux distributions but the `_crypt` standard library module depends on it.
Depending on the Linux distributions you need to run different commands to
resolve this:

* archlinux: `pacman -S libxcrypt-compat`
* CentOS/RedHat: `dnf install libxcrypt-compat`

There have also been reports of an error being generated at installation time
despite `libcrypt.so.1` being installed when a different `ldd` (eg: Homebrew)
shadows the system one.  In that case try the installation again after giving
the default one higher priority in the `PATH:

```
export PATH="/usr/bin:$PATH"
curl -sSf https://rye-up.com/get | bash
```

## References to Build-Time Paths

The prefers using standalone Python builds.  As Python historically is not much
accommodating to portable builds there are various limitations still with this
approach.  One of them is that built Python distributions capture some absolute
paths and other build-time configuration.  These file paths are then often used
by build tools to invoke C compilers.  For instance you might run into a compiler
error like ``error: stdio.h: No such file or directory`` when building C
extensions.  There is no known solution to this problem today other than
[registering a non portable toolchain](toolchains/index.md#registering-toolchains).

This issue is inherited from `python-build-standalone` and more information can
be found in the documentation: [References to Build-Time Paths](https://gregoryszorc.com/docs/python-build-standalone/main/quirks.html#references-to-build-time-paths).  There is also an open 
Rye issue for it: [Issue #621](https://github.com/astral-sh/rye/issues/621).

## TKinter Support

TKinter uses TCL behind the scenes.  Unfortunately this also means that some runtime
support is required.  This runtime support is provided by the portable Python builds,
however the way TCL is initialized on macOS and Linux won't find these files in
virtualenvs.  Newer versions of Rye will automatically export the `TCL_LIBRARY`
and `TK_LIBRARY` environment variables for you in a manner very similar to this:

```python
import os
import sys
os.environ["TCL_LIBRARY"] = sys.base_prefix + "/lib/tcl8.6"
os.environ["TK_LIBRARY"] = sys.base_prefix + "/lib/tk8.6"
```

## Python Interactive Prompt Input Messed Up

The Python builds that Rye uses are compiled against `libedit` rather than `readline`
for licensing reasons.  You might run into unicode issues on input as a result of this
due to limitations in `libedit`.  In some cases though you might also discover that
the backspace key does not work or arrow keys don't work as expected.  This can be
because the _terminfo_ database cannot be found.

For solutions to this issue, read the [behavior quirks guide](https://python-build-standalone.readthedocs.io/en/latest/quirks.html) in the
Standalone Python Builds documentation for solutions.

## Can I use Rye Alongside Other Python Installations?

Rye given it's experimental nature does not want to disrupt already existing Python
workflows.  As such using it alongside other Python installations is intentionally
supported.  Even if the Rye shims come first on the `PATH`, Rye will automatically
resolve to a different Python installation on the search path when invoked in a
folder that contains a non Rye managed project.

As such the answer is a clear **yes!**

## Musl/Alpine Support

When bootstrapping it can happen that you are running into a confusing error like
"No such file or directory (os error 2)".  This can happen on MUSL based Linux
systems like Alpine.  The reason for this is that Rye downloads distribution
independent Python interpreters which are not compatible with Linux systems that
do not use glibc.  The solution today is to install Python via other means and
to install Rye with a custom `RYE_TOOLCHAIN`.  For more information see
[Customized Installation](/guide/installation/#customized-installation)

## Wheels Appear to be Missing Files

You might be encountering missing files in wheels when running `rye build` and you
are using hatchling.  The reason for this is that `rye build` uses
"[build](https://pypi.org/project/build/)" behind the scenes to build wheels.  There
are two build modes and in some cases the wheel is first built from an sdist.  So
if your sdists does not include the necessary data files, the resulting wheel will
also be incorrect.

This can be corrected by adding the files to the `include` in the hatch config
for sdists.  For instance the following lines added to `pyproject.toml` will add
the data files in `my_package` and all the tests to the sdist from which the
wheel is built:

```toml
[tool.hatch.build.targets.sdist]
include = ["src/my_package", "tests"]
```

## Can I Relocate Virtualenvs?

Rye very intentionally places the virtualenv (`.venv`) in the root folder of the
workspace.  Relocations of virtualenvs is not supported.  This is a very intentional
decision so that tools do not need to deal with various complex alternatives and can
rely on a simple algorithm to locate it.  This is a form of convention over configuration
and can also assist editor integrations.

There are some known downsides of this.  For instance if you are placing your projects
in Dropbox, it would cause this folder to synchronize.  As a way to combat this, Rye
will automatically mark the virtualenv with the necessary flags to disable cloud sync
of known supported cloud synchronization systems.

For override this behavior you can set the `behavior.venv-mark-sync-ignore` configuration
key to `false`.

## Why Does Rye Contain Trojan "Bearfoos"?

Unfortunately Windows likes to complain that Rye contains the trojan "Win32/Bearfoos.A!ml".
This seems to be something that happens to a few programs written in Rust every once in a
while because the compiler spits out some bytes that have been associated with Trojans
written in Rust.

It can be ignored.  For more information see the discussion [Windows Bearfoos
virus associated with rye](https://github.com/astral-sh/rye/issues/468).
