---
hide:
  - navigation
---

# Rye: A New Kind Of Package Management Solution for Python

Rye is a project and package management solution for Python, created by
[Armin](https://github.com/mitsuhiko/).  It came out of his desire to create a
one-stop-shop for all Python needs.  It installs and manages Python
installations, manages `pyproject.toml` files, installs and uninstalls
dependencies, manages virtualenvs behind the scenes.  It supports monorepos,
global tool installations.

Rye is an experimental endeavour to build a new type of packaging experience to
Python inspired by `rustup` and `cargo` from Rust.  Please give it a try.
Feedback and suggestions are greatly appreciated.

**What it does:**

* Download and manage Python interpreters
* Manage Projects
* Manage your dependencies via [pip-tools](https://github.com/jazzband/pip-tools)
* [Manage your virtualenvs](guide/sync.md)
* Simple way to [invoke scripts](guide/pyproject.md#toolryescripts)
* Lint and format via [Ruff](https://astral.sh/ruff)

<script async defer src="https://buttons.github.io/buttons.js"></script>
<p align="center">
  <a class="github-button" href="https://github.com/mitsuhiko/rye" data-size="large" data-show-count="true" data-color-scheme="light" aria-label="Star mitsuhiko/insta on GitHub">Star</a>
<a class="github-button" href="https://github.com/mitsuhiko/rye/discussions" data-icon="octicon-comment-discussion" data-size="large" aria-label="Discuss mitsuhiko/rye on GitHub">Discuss</a>
  <a class="github-button" href="https://github.com/sponsors/mitsuhiko" data-size="large" data-icon="octicon-heart" data-color-scheme="light" aria-label="Sponsor @mitsuhiko on GitHub">Sponsor</a>
</p>

!!! abstract "Installation Instructions"

    {% include-markdown ".includes/quick-install.md" %}

    For the next steps or ways to customize the installation, head over to the detailed
    [installation](./guide/installation.md) guide.