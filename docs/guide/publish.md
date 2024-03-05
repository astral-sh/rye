# Building and Publishing

Rye currently uses [build](https://github.com/pypa/build) to build the package and uses [twine](https://github.com/pypa/twine) to publish it.

## Build

By default, `rye` will build both the sdist and wheel targets in the `dist` directory.   The command for this is called [`build`](commands/build.md).

```
rye build
```

You can use the `--sdist` or `--wheel` flag to build the specific target, or specify the output directory with `--out`.

```
rye build --wheel --out target
```

If you want to clean the build directory before building, run:

```
rye build --clean
```

## Publish

Rye will publish the distribution files under the `dist` directory to PyPI by default.

```bash
rye publish
```

You might be asked to input your access token and some other info if needed.

```
No access token found, generate one at: https://pypi.org/manage/account/token/
Access token:

```

You can also specify the distribution files to be published:

```
rye publish dist/example-0.1.0.tar.gz
```

### --repository

Rye supports publishing the package to a different repository by using the `--repository` and `--repository-url` flags. For example, to publish to the test PyPI repository:

```
rye publish --repository testpypi --repository-url https://test.pypi.org/legacy/
```

### --yes

You can optionally set the `--yes` flag to skip the confirmation prompt. This can be useful for CI/CD pipelines.

```
rye publish --token <your_token> --yes
```

Rye will store your repository info in `$HOME/.rye/credentials` for future use.

### --skip-existing

You can use `--skip-existing` to skip any distribution files that have already been published to the repository. Note that some repositories may not support this feature.
