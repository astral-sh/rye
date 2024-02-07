`RYE_TOOLCHAIN`

:   Optionally this environment variable can be set to point to a Python
    interpreter that should be used as the internal interpreter.  If not
    provided a suitable interpreter is automatically downloaded.

    At present only CPython 3.9 to 3.12 are supported.

`RYE_TOOLCHAIN_VERSION`

:   For Rye 0.22 and later a specific Python version can be picked rather
    than the default.  This affects the internal toolchain version only.
    It's useful for Docker builds where you can set the internal toolchain
    to the same as your project to only fetch a single Python.

    At present only CPython 3.9 to 3.12 are supported.
