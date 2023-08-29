# Rust Modules

Rye recommends using [maturin](https://github.com/PyO3/maturin) to develop Rust Python
extension modules.  This process is largely automated and new projects can be created
with `rye init`.

## New Project

```
rye init my-project --build-system maturin
cd maturin
```

The following structure will be created:

```
.
├── .git
├── .gitignore
├── .python-version
├── README.md
├── pyproject.toml
├── Cargo.toml
└── src
    └── lib.rs
``` 

## Working with Maturin

When you use maturin as a build system then `rye sync` will automatically build the rust
extension module into your venv.  Likewise `rye build` will us maturin to trigger a
wheel build.

If you want to use other maturin commands such as `maturin develop` you can install
it as a global tool:

```
rye install maturin
```

Note that `maturin develop`` requires `pip` to be installed into the virtualenv.  Before
you can use it you need to add it:

```
rye add --dev pip
rye sync
```
