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

## Python Interactive Prompt Input Messed Up

The Python builds that Rye uses are compiled against `libedit` rather than `readline`
for licensing reasons.  You might run into unicode issues on input as a result of this
due to limitations in `libedit`.  In some cases though you might also discover that
the backspace key does not work or arrow keys don't work as expected.  This can be
because the _terminfo_ database cannot be found.

For solutions to this issue, read the [behavior quirks guide](https://python-build-standalone.readthedocs.io/en/latest/quirks.html) in the
Standalone Python Builds documentation for solutions.
