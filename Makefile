.PHONY: all
all: test

.PHONY: build
build:
	@cargo build --all

.PHONY: test
test:
	@cargo insta test --workspace --all-features

.PHONY: check
check:
	@cargo check --all

.PHONY: format
format:
	@cargo fmt --all

.PHONY: format-check
format-check:
	@cargo fmt --all -- --check

.PHONY: serve-docs
serve-docs: .venv
	@rye run serve-docs

.PHONY: lint
lint:
	@cargo clippy --all -- -D clippy::dbg-macro -D warnings

.venv:
	@rye sync

.PHONY: sync-python-releases
sync-python-releases: .venv
	@rye run find-downloads > rye/src/sources/generated/python_downloads.inc

.PHONY: sync-uv-releases
sync-uv-releases: .venv
	@rye run uv-downloads > rye/src/sources/generated/uv_downloads.inc
