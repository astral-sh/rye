# FAQ

This section should cover some commonly asked questions.  If you do not find an answer
here, consider reaching out to the [community](../community.md).

## Windows Developer Mode

Rye does not require symlinks but it works significantly better with them.  On Windows
support for symlinks is restricted to privileged accounts.  The reason for this is that
Symlinks were a late addition to Windows and some applications are not developed with
them in mind which can cause misbehavior or in the worst case security issues in those
applications.  Symlinks support however is enabled when the "developer mode" is activated
on modern Windows versions.  Here is how you can enable it:

1. Press ++windows+i++ to open the settings
2. In the settings dialog click on "Privacy & security"
3. In the "Security" section click on "For developers"
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
