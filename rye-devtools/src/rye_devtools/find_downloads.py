"""This script is used to generate rye/src/sources/generated/python_downloads.inc.

It finds the latest Python releases, sorts them by
various factors (arch, platform, flavor) and generates download
links to be included into rye at build time.
"""

import abc
import asyncio
import os
import re
import sys
from dataclasses import dataclass
from enum import StrEnum
from urllib.parse import unquote

import httpx
from httpx import HTTPStatusError

from .common import PlatformTriple, Version, batched, fetch, log


class PythonImplementation(StrEnum):
    CPYTHON = "cpython"
    PYPY = "pypy"


@dataclass
class PythonDownload:
    version: Version
    triple: PlatformTriple
    implementation: PythonImplementation
    filename: str
    url: str
    sha256: str | None = None


class Finder:
    implementation: PythonImplementation

    @abc.abstractmethod
    async def find(self) -> list[PythonDownload]:
        raise NotImplementedError


class CPythonFinder(Finder):
    implementation = PythonImplementation.CPYTHON

    RELEASE_URL = (
        "https://api.github.com/repos/indygreg/python-build-standalone/releases"
    )

    FLAVOR_PREFERENCES = [
        "shared-pgo",
        "shared-noopt",
        "shared-noopt",
        "pgo+lto",
        "pgo",
        "lto",
    ]
    HIDDEN_FLAVORS = [
        "debug",
        "noopt",
        "install_only",
    ]
    SPECIAL_TRIPLES = {
        "macos": "x86_64-apple-darwin",
        "linux64": "x86_64-unknown-linux-gnu",
        "windows-amd64": "x86_64-pc-windows-msvc",
        "windows-x86-shared-pgo": "i686-pc-windows-msvc-shared-pgo",
        "windows-amd64-shared-pgo": "x86_64-pc-windows-msvc-shared-pgo",
        "windows-x86": "i686-pc-windows-msvc",
        "linux64-musl": "x86_64-unknown-linux-musl",
        "linux64-riscv64": "riscv64-unknown-linux-gnu",
    }

    # matches these: https://doc.rust-lang.org/std/env/consts/constant.ARCH.html
    ARCH_MAPPING = {
        "x86_64": "x86_64",
        "x86": "x86",
        "i686": "x86",
        "aarch64": "aarch64",
        "riscv64": "riscv64",
    }

    # matches these: https://doc.rust-lang.org/std/env/consts/constant.OS.html
    PLATFORM_MAPPING = {
        "darwin": "macos",
        "windows": "windows",
        "linux": "linux",
    }

    ENV_MAPPING = {
        "gnu": "gnu",
        # We must ignore musl for now
        # "musl": "musl",
    }

    FILENAME_RE = re.compile(
        r"""(?x)
    ^
        cpython-(?P<ver>\d+\.\d+\.\d+?)
        (?:\+\d+)?
        -(?P<triple>.*?)
        (?:-[\dT]+)?\.tar\.(?:gz|zst)
    $
"""
    )

    def __init__(self, client: httpx.AsyncClient):
        self.client = client

    async def find(self) -> list[PythonDownload]:
        downloads = await self.fetch_indygreg_downloads()
        await self.fetch_indygreg_checksums(downloads, n=20)
        return downloads

    async def fetch_indygreg_downloads(self, pages: int = 100) -> list[PythonDownload]:
        """Fetch all the indygreg downloads from the release API."""
        results: dict[Version, dict[tuple[str, str], list[PythonDownload]]] = {}

        for page in range(1, pages):
            log(f"Fetching indygreg release page {page}")
            resp = await fetch(self.client, "%s?page=%d" % (self.RELEASE_URL, page))
            rows = resp.json()
            if not rows:
                break
            for row in rows:
                for asset in row["assets"]:
                    url = asset["browser_download_url"]
                    download = self.parse_download_url(url)
                    if download is not None:
                        (
                            results.setdefault(download.version, {})
                            # For now, we only group by arch and platform, because Rust's PythonVersion doesn't have a notion
                            # of environment. Flavor will never be used to sort download choices and must not be included in grouping.
                            .setdefault(
                                (download.triple.arch, download.triple.platform), []
                            )
                            .append(download)
                        )

        downloads = []
        for version, platform_downloads in results.items():
            for flavors in platform_downloads.values():
                best = self.pick_best_download(flavors)
                if best is not None:
                    downloads.append(best)
        return downloads

    @classmethod
    def parse_download_url(cls, url: str) -> PythonDownload | None:
        """Parse an indygreg download URL into a PythonDownload object."""
        # The URL looks like this:
        # https://github.com/indygreg/python-build-standalone/releases/download/20240107/cpython-3.12.1%2B20240107-aarch64-unknown-linux-gnu-lto-full.tar.zst
        filename = unquote(url.rsplit("/", maxsplit=1)[-1])
        if filename.endswith(".sha256"):
            return

        match = cls.FILENAME_RE.match(filename)
        if match is None:
            return

        version_str, triple_str = match.groups()
        version = Version.from_str(version_str)
        triple = cls.parse_triple(triple_str)
        if triple is None:
            return

        return PythonDownload(
            version=version,
            triple=triple,
            implementation=PythonImplementation.CPYTHON,
            filename=filename,
            url=url,
        )

    @classmethod
    def parse_triple(cls, triple: str) -> PlatformTriple | None:
        """Parse a triple into a PlatformTriple object."""

        def match_flavor(triple: str) -> str | None:
            for flavor in cls.FLAVOR_PREFERENCES + cls.HIDDEN_FLAVORS:
                if flavor in triple:
                    return flavor
            return None

        def match_mapping(
            pieces: list[str], mapping: dict[str, str]
        ) -> tuple[str | None, list[str]]:
            for i in reversed(range(0, len(pieces))):
                if pieces[i] in mapping:
                    return mapping[pieces[i]], pieces[:i]
            return None, pieces

        # Map, old, special triplets to proper triples for parsing, or
        # return the triple if it's not a special one
        triple = cls.SPECIAL_TRIPLES.get(triple, triple)

        # freethreaded builds are experimental, ignore them for now
        if "freethreaded" in triple:
            return

        pieces = triple.split("-")
        flavor = match_flavor(triple)
        env, pieces = match_mapping(pieces, cls.ENV_MAPPING)
        platform, pieces = match_mapping(pieces, cls.PLATFORM_MAPPING)
        arch, pieces = match_mapping(pieces, cls.ARCH_MAPPING)

        if arch is None or platform is None:
            return

        if env is None and platform == "linux":
            return

        return PlatformTriple(arch, platform, env, flavor)

    @classmethod
    def pick_best_download(
        cls, downloads: list[PythonDownload]
    ) -> PythonDownload | None:
        """Pick the best download from the list of downloads."""

        def preference(download: PythonDownload) -> int:
            try:
                return cls.FLAVOR_PREFERENCES.index(download.triple.flavor)
            except ValueError:
                return len(cls.FLAVOR_PREFERENCES) + 1

        downloads.sort(key=preference)
        return downloads[0] if downloads else None

    async def fetch_indygreg_checksums(
        self, downloads: list[PythonDownload], n: int = 10
    ) -> None:
        """Fetch the checksums for the given downloads."""
        checksums_url = set()
        for download in downloads:
            release_url = download.url.rsplit("/", maxsplit=1)[0]
            checksum_url = release_url + "/SHA256SUMS"
            checksums_url.add(checksum_url)

        async def fetch_checksums(url: str):
            try:
                resp = await fetch(self.client, url)
            except HTTPStatusError as e:
                if e.response.status_code != 404:
                    raise
                return None
            return resp

        completed = 0
        tasks = []
        for batch in batched(checksums_url, n):
            log(f"Fetching indygreg checksums: {completed}/{len(checksums_url)}")
            async with asyncio.TaskGroup() as tg:
                for url in batch:
                    task = tg.create_task(fetch_checksums(url))
                    tasks.append(task)
            completed += n

        checksums = {}
        for task in tasks:
            resp = task.result()
            if resp is None:
                continue
            lines = resp.text.splitlines()
            for line in lines:
                checksum, filename = line.split(" ", maxsplit=1)
                filename = filename.strip()
                checksums[filename] = checksum

        for download in downloads:
            download.sha256 = checksums.get(download.filename)


class PyPyFinder(Finder):
    implementation = PythonImplementation.PYPY

    RELEASE_URL = "https://raw.githubusercontent.com/pypy/pypy/main/pypy/tool/release/versions.json"
    CHECKSUM_URL = (
        "https://raw.githubusercontent.com/pypy/pypy.org/main/pages/checksums.rst"
    )
    CHECKSUM_RE = re.compile(
        r"^\s*(?P<checksum>\w{64})\s+(?P<filename>pypy.+)$", re.MULTILINE
    )

    ARCH_MAPPING = {
        "x64": "x86_64",
        "i686": "x86",
        "aarch64": "aarch64",
        "arm64": "aarch64",
    }

    PLATFORM_MAPPING = {
        "darwin": "macos",
        "win64": "windows",
        "linux": "linux",
    }

    def __init__(self, client: httpx.AsyncClient):
        self.client = client

    async def find(self) -> list[PythonDownload]:
        downloads = await self.fetch_downloads()
        await self.fetch_checksums(downloads)
        return downloads

    async def fetch_downloads(self) -> list[PythonDownload]:
        log("Fetching pypy downloads...")
        resp = await fetch(self.client, self.RELEASE_URL)
        versions = resp.json()

        results = {}
        for version in versions:
            if not version["stable"]:
                continue
            python_version = Version.from_str(version["python_version"])
            if python_version < (3, 7, 0):
                continue
            for file in version["files"]:
                arch = self.ARCH_MAPPING.get(file["arch"])
                platform = self.PLATFORM_MAPPING.get(file["platform"])
                if arch is None or platform is None:
                    continue
                environment = "gnu" if platform == "linux" else None
                download = PythonDownload(
                    version=python_version,
                    triple=PlatformTriple(
                        arch=arch,
                        platform=platform,
                        environment=environment,
                        flavor=None,
                    ),
                    implementation=PythonImplementation.PYPY,
                    filename=file["filename"],
                    url=file["download_url"],
                )
                # Only keep the latest pypy version of each arch/platform
                if (python_version, arch, platform) not in results:
                    results[(python_version, arch, platform)] = download

        return list(results.values())

    async def fetch_checksums(self, downloads: list[PythonDownload]) -> None:
        log("Fetching pypy checksums...")
        resp = await fetch(self.client, self.CHECKSUM_URL)
        text = resp.text

        checksums = {}
        for match in self.CHECKSUM_RE.finditer(text):
            checksums[match.group("filename")] = match.group("checksum")

        for download in downloads:
            download.sha256 = checksums.get(download.filename)


def render(downloads: list[PythonDownload]):
    """Render downloads.inc."""

    def sort_key(download: PythonDownload) -> tuple[int, Version, PlatformTriple]:
        # Sort by implementation, version (latest first), and then by triple.
        impl_order = [PythonImplementation.PYPY, PythonImplementation.CPYTHON]
        return (
            impl_order.index(download.implementation),
            -download.version,
            download.triple,
        )

    downloads.sort(key=sort_key)

    print("// Generated by rye-devtools. DO NOT EDIT.")
    print(
        "// To regenerate, run `rye run find-downloads > rye/src/sources/generated/python_downloads.inc` from the root of the repository."
    )
    print("use std::borrow::Cow;")
    print("pub const PYTHON_VERSIONS: &[(PythonVersion, &str, Option<&str>)] = &[")

    for download in downloads:
        triple = download.triple
        version = download.version
        sha256 = f'Some("{download.sha256}")' if download.sha256 else "None"
        print(
            f'    (PythonVersion {{ name: Cow::Borrowed("{download.implementation}"), arch: Cow::Borrowed("{triple.arch}"), os: Cow::Borrowed("{triple.platform}"), major: {version.major}, minor: {version.minor}, patch: {version.patch}, suffix: None }}, "{download.url}", {sha256}),'
        )

    print("];")


async def async_main():
    token = os.environ.get("GITHUB_TOKEN")
    if not token:
        try:
            token = open("token.txt").read().strip()
        except Exception:
            pass

    if not token:
        log("Please set GITHUB_TOKEN environment variable or create a token.txt file.")
        sys.exit(1)

    headers = {
        "X-GitHub-Api-Version": "2022-11-28",
        "Authorization": "Bearer " + token,
    }
    client = httpx.AsyncClient(follow_redirects=True, headers=headers)

    finders = [
        CPythonFinder(client),
        PyPyFinder(client),
    ]
    downloads = []

    log("Fetching all Python downloads and generating code.")
    async with client:
        for finder in finders:
            log(f"Finding {finder.implementation} downloads...")
            downloads.extend(await finder.find())

    render(downloads)


def main():
    asyncio.run(async_main())


if __name__ == "__main__":
    main()
