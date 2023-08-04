# Reformatting

You can reformat all the code using

```
rye fmt
```

This uses the popular [Black](https://black.readthedocs.io/en/stable)
autoformatter.

You can pass arbitrary arguments to `black` by separating them from
`rye fmt` arguments with `--`, e.g.,

```
rye fmt -- --line-length 80
```

You can also configure Black in the `pyproject.toml` file.  See the
[Black documentation on configuration](https://black.readthedocs.io/en/stable/usage_and_configuration/the_basics.html#configuration-via-a-file)
for more.
