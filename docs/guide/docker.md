# Building a Container with Docker

If you want to put your Python code into a container, you probably have some server code that you don't submit to PyPI or another registry.
If that's the case, read on. Else, skip to [the next section](#container-from-a-python-package).

This guide requires some familiarity with Docker and Dockerfiles.

## Container from Source

1. Make sure that your project is set up as a [virtual project](./virtual.md).
     This means that you can't install it, and it won't mark itself as a dependency.
     If you need your project to be installable, go to [the next section](#container-from-a-python-package).
  
     - Your `pyproject.toml` should contain `virtual = true` under the `[tool.rye]` section. If it's not there, add it and run `rye sync`.
     - If you're just setting up a project, run `rye init --virtual` instead of `rye init`.

2. Create a `Dockerfile` in your project root with the following content, using [`uv`](https://github.com/astral-sh/uv):
    
    ```Dockerfile
    FROM python:slim
    
    RUN pip install uv

    WORKDIR /app
    COPY requirements.lock ./
    RUN uv pip install --no-cache --system -r requirements.lock
    
    COPY src .
    CMD python main.py
    ```

    Or, using `pip`:

    ```Dockerfile
    FROM python:slim
    
    WORKDIR /app
    COPY requirements.lock ./
    RUN PYTHONDONTWRITEBYTECODE=1 pip install --no-cache-dir -r requirements.lock
    
    COPY src .
    CMD python main.py
    ```

3. You can now build your image like this:
   
    ```bash
    docker build .
    ```

### Dockerfile Adjustments

The `Dockerfile`s in this guide are examples. Some adjustments you might want to make:

- The command (`CMD python src/main.py`) should point to your script.
- Adjust the base image (`FROM python:slim`):
  - Prefer a tagged version that matches the one from your `.python-version` file, e.g. `FROM python:3.12.0-slim`.
  - The `-slim` variants are generally a good tradeoff between image size and compatibility and should work fine for most workloads. 
  But you can also use `-alpine` for smaller images (but potential compatibility issues) or no suffix for ones that contain more system tools.
- If you need additional system packages, install them before copying your source code, i.e. before the line `COPY src .`.
  When using Debian-based images (i.e. `-slim` or no-suffix variants), that could look like this:

  ```Dockerfile
  RUN apt-get update \
      && apt-get install -y --no-install-recommends some-dependency another-dependency \
      && rm -rf /var/lib/apt/lists/*
  ```

## Container from a Python Package

If your code is an installable package, it's recommended that you first build it, then install it inside your Docker image.
This way you can be sure that the image is exactly the same as what a user installation would be.

An example `Dockerfile` might look like this with [`uv`](https://github.com/astral-sh/uv):

```Dockerfile
FROM python:slim
RUN pip install uv
RUN --mount=source=dist,target=/dist uv pip install --no-cache /dist/*.whl
CMD python -m my_package
```

To build your docker image, you'll have to first build your wheel, like this:

```bash
rye build --wheel --clean
docker build . --tag your-image-name
```

Note that this approach bundles your dependencies and code in a single layer.
This might be nice for performance, but it also means that all dependencies are re-installed during every image build, and different versions won't share the disk space for the dependencies.

The [Dockerfile adjustments from the previous section](#dockerfile-adjustments) apply.

## Explanations

Rye's lockfile standard is the `requirements.txt` format from `pip` (and used by [`uv`](https://github.com/astral-sh/uv)), so you don't actually need `rye` in your container to be able to install dependencies.
This makes the Dockerfile much simpler and avoids the necessity for multi-stage builds if small images are desired.

The `--no-cache-dir` and `--no-cache` parameters, passed to `pip` and `uv` respectively, make the image smaller by not
writing any temporary files. While caching can speed up subsequent builds, it's not necessary in a container where the
image is built once and then used many times.

Similarly, the `PYTHONDONTWRITEBYTECODE=1` environment variable is set to avoid writing `.pyc` files, which are not
needed in a container. (`uv` skips writing `.pyc` files by default.) 
