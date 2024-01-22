# `install`

Installs a package as global tool.  This command has two names
to `rye tools install` and `rye install`.

This can be used to install useful Python scripts globally into it's own
separated virtualenv.  For instance if you want to use the `black` formatter
you can install it once.

Normally only scripts installed by the top level dependency are installed.  In
some cases you might also want to install commands from sub-dependencies.  In
that case pass those dependencies with `--include-dep`.

For more information see [Tools](/guide/tools/).

## Example

```
$ rye tools install pycowsay
Looking in indexes: https://pypi.org/simple/
Collecting pycowsay
  Downloading pycowsay-0.0.0.2-py3-none-any.whl.metadata (965 bytes)
Downloading pycowsay-0.0.0.2-py3-none-any.whl (4.0 kB)
Installing collected packages: pycowsay
Successfully installed pycowsay-0.0.0.2

Installed scripts:
  - pycowsay

$ pycowsay "Great Stuff"

  -----------
< Great Stuff >
  -----------
   \   ^__^
    \  (oo)\_______
       (__)\       )\/\
           ||----w |
           ||     ||
```

## Arguments

* `<REQUIREMENT>...`: The package to install as PEP 508 requirement string.

## Options

* `--git <GIT>`: Install the given package from this git repository

* `--url <URL>`: Install the given package from this URL

* `--path <PATH>`: Install the given package from this local path

* `--absolute`: Force non interpolated absolute paths

* `--tag <TAG>`: Install a specific tag

* `--rev <REV>`: Update to a specific git rev

* `--branch <BRANCH>`: Update to a specific git branch

* `--features <FEATURES>`: Adds a dependency with a specific feature

* `--include-dep <INCLUDE_DEP>`: Include scripts from a given dependency

* `--extra-requirement <EXTRA_REQUIREMENT>`: Additional dependencies to install that are not declared by the main package

* `-p, --python <PYTHON>`: Optionally the Python version to use

* `-f, --force`: Force install the package even if it's already there

* `-v, --verbose`: Enables verbose diagnostics

* `-q, --quiet`: Turns off all output

* `-h, --help`: Print help (see a summary with '-h')