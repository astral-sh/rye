"""This script is used to generate rye/src/downloads.inc.

It find the latest python-build-standalone releases, sorts them by
various factors (arch, platform, flavor) and generates download
links to be included into rye at build time.  In addition it maintains
a manual list of pypy downloads to be included into rye at build
time.
"""
import re
import sys
import time
import unittest
from dataclasses import dataclass
from datetime import datetime, timezone
from enum import Enum
from itertools import chain
from typing import Callable, Optional, Self
from urllib.parse import unquote

import requests


def log(*args, **kwargs):
    print(*args, file=sys.stderr, **kwargs)


SESSION = requests.Session()
TOKEN = open("token.txt").read().strip()
RELEASE_URL = "https://api.github.com/repos/indygreg/python-build-standalone/releases"
HEADERS = {
    "X-GitHub-Api-Version": "2022-11-28",
    "Authorization": "Bearer " + TOKEN,
}
FLAVOR_PREFERENCES = [
    "shared-pgo",
    "shared-noopt",
    "shared-noopt",
    "pgo+lto",
    "lto",
    "pgo",
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
}

# matches these: https://doc.rust-lang.org/std/env/consts/constant.ARCH.html
ARCH_MAPPING = {
    "x86_64": "x86_64",
    "x86": "x86",
    "i686": "x86",
    "aarch64": "aarch64",
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


@dataclass(frozen=True)
class PlatformTriple:
    arch: str
    platform: str
    environment: Optional[str]
    flavor: str

    @classmethod
    def from_str(cls, triple: str) -> Optional[Self]:
        """Parse a triple into a PlatformTriple object."""

        # The parsing functions are all very similar and we could abstract them into a single function
        # but I think it's clearer to keep them separate.
        def match_flavor(triple):
            for flavor in FLAVOR_PREFERENCES + HIDDEN_FLAVORS:
                if flavor in triple:
                    return flavor
            return ""

        def match_mapping(pieces: list[str], mapping: dict[str, str]):
            for i in reversed(range(0, len(pieces))):
                if pieces[i] in mapping:
                    return mapping[pieces[i]], pieces[:i]
            return None, pieces

        # We split by '-' and match back to front to extract the flavor, env, platform and archk
        arch, platform, env, flavor = None, None, None, None

        # Map, old, special triplets to proper triples for parsing, or
        # return the triple if it's not a special one
        triple = SPECIAL_TRIPLES.get(triple, triple)
        pieces = triple.split("-")
        flavor = match_flavor(triple)
        env, pieces = match_mapping(pieces, ENV_MAPPING)
        platform, pieces = match_mapping(pieces, PLATFORM_MAPPING)
        arch, pieces = match_mapping(pieces, ARCH_MAPPING)

        if flavor is None or arch is None or platform is None:
            return

        if env is None and platform == "linux":
            return

        return cls(arch, platform, env, flavor)

    def grouped(self) -> tuple[str, str]:
        # for now we only group by arch and platform, because rust's PythonVersion doesn't have a notion
        # of environment. Flavor will never be used to sort download choices and must not be included in grouping.
        return self.arch, self.platform
        # return self.arch, self.platform, self.environment or ""


@dataclass(frozen=True, order=True)
class PythonVersion:
    major: int
    minor: int
    patch: int

    @classmethod
    def from_str(cls, version: str) -> Self:
        return cls(*map(int, version.split(".", 3)))


@dataclass(frozen=True)
class IndygregDownload:
    version: PythonVersion
    triple: PlatformTriple
    url: str

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

    @classmethod
    def from_url(cls, url) -> Optional[Self]:
        base_name = unquote(url.rsplit("/")[-1])
        if base_name.endswith(".sha256"):
            return

        match = cls.FILENAME_RE.match(base_name)
        if match is None:
            return

        # Parse version string and triplet string
        version_str, triple_str = match.groups()
        version = PythonVersion.from_str(version_str)
        triple = PlatformTriple.from_str(triple_str)
        if triple is None:
            return

        return cls(version, triple, url)

    def sha256(self) -> Optional[str]:
        """We only fetch the sha256 when needed. This generally is AFTER we have
        decided that the download will be part of rye's download set"""
        resp = fetch(self.url + ".sha256", headers=HEADERS)
        if not resp.ok:
            return None
        return resp.text.strip()


def fetch(page, headers):
    """Fetch a page from GitHub API with ratelimit awareness."""
    resp = SESSION.get(page, headers=headers, timeout=90)
    if (
        resp.status_code in [403, 429]
        and resp.headers.get("x-ratelimit-remaining") == "0"
    ):
        # See https://docs.github.com/en/rest/using-the-rest-api/troubleshooting-the-rest-api?apiVersion=2022-11-28
        if (retry_after := resp.headers.get("retry-after")) is not None:
            log("got retry-after header. retrying in {retry_after} seconds.")
            time.sleep(int(retry_after))

            return fetch(page, headers)

        if (retry_at := resp.headers.get("x-ratelimit-reset")) is not None:
            utc = datetime.now(timezone.utc).timestamp()
            retry_after = int(retry_at) - int(utc)

            log("got x-ratelimit-reset header. retrying in {retry_after} seconds.")
            time.sleep(max(int(retry_at) - int(utc), 0))

            return fetch(page, headers)

        log("got rate limited but no information how long. waiting for 2 minutes")
        time.sleep(60 * 2)
        return fetch(page, headers)
    return resp


def fetch_indiygreg_downloads(
    pages: int = 100,
) -> dict[PythonVersion, dict[PlatformTriple, list[IndygregDownload]]]:
    """Fetch all the indygreg downloads from the release API."""
    results = {}

    for page in range(1, pages):
        log(f"Fetching page {page}")
        resp = fetch("%s?page=%d" % (RELEASE_URL, page), headers=HEADERS)
        rows = resp.json()
        if not rows:
            break
        for row in rows:
            for asset in row["assets"]:
                url = asset["browser_download_url"]
                if (download := IndygregDownload.from_url(url)) is not None:
                    results.setdefault(download.version, {}).setdefault(download.triple.grouped(), []).append(download)
    return results


def pick_best_download(downloads: list[IndygregDownload]) -> Optional[IndygregDownload]:
    """Pick the best download from the list of downloads."""

    def preference(download: IndygregDownload) -> int:
        try:
            return FLAVOR_PREFERENCES.index(download.triple.flavor)
        except ValueError:
            return len(FLAVOR_PREFERENCES) + 1

    downloads.sort(key=preference)
    return downloads[0] if downloads else None


def render(
    indys: dict[PythonVersion, list[IndygregDownload]],
    pypy: dict[PythonVersion, dict[PlatformTriple, str]],
):
    """Render downloads.inc"""
    log("Generating code and fetching sha256 of all cpython downloads.")
    log("This can be slow......")

    print("// generated code, do not edit")
    print("use std::borrow::Cow;")
    print("pub const PYTHON_VERSIONS: &[(PythonVersion, &str, Option<&str>)] = &[")

    for version, downloads in sorted(pypy.items(), key=lambda v: v[0], reverse=True):
        for triple, url in sorted(downloads.items(), key=lambda v: v[0].grouped()):
            print(
                f'    (PythonVersion {{ name: Cow::Borrowed("pypy"), arch: Cow::Borrowed("{triple.arch}"), os: Cow::Borrowed("{triple.platform}"), major: {version.major}, minor: {version.minor}, patch: {version.patch}, suffix: None }}, "{url}", None),'
            )

    for version, downloads in sorted(indys.items(), key=lambda v: v[0], reverse=True):
        for download in sorted(downloads, key=lambda v: v.triple.grouped()):
            if (sha256 := download.sha256()) is not None:
                sha256_str = f'Some("{sha256}")'
            else:
                sha256_str = "None"
            print(
                f'    (PythonVersion {{ name: Cow::Borrowed("cpython"), arch: Cow::Borrowed("{download.triple.arch}"), os: Cow::Borrowed("{download.triple.platform}"), major: {version.major}, minor: {version.minor}, patch: {version.patch}, suffix: None }}, "{download.url}", {sha256_str}),'
            )
    print("];")


def main():
    log("Rye download creator started.")
    log("Fetching indygreg downloads...")

    indys = {}
    # For every version, pick the best download per triple
    # and store it in the results
    for version, download_choices in fetch_indiygreg_downloads(100).items():
        # Create a dict[PlatformTriple, list[IndygregDownload]]]
        # for each version
        for triple, choices in download_choices.items():
            if (best_download := pick_best_download(choices)) is not None:
                indys.setdefault(version, []).append(best_download)

    render(indys, PYPY_DOWNLOADS)


# These are manually maintained for now
PYPY_DOWNLOADS = {
    PythonVersion(3, 10, 12): {
        PlatformTriple(
            arch="x86_64", platform="linux", environment="gnu", flavor=""
        ): "https://downloads.python.org/pypy/pypy3.10-v7.3.12-linux64.tar.bz2",
        PlatformTriple(
            arch="aarch64", platform="linux", environment="gnu", flavor=""
        ): "https://downloads.python.org/pypy/pypy3.10-v7.3.12-aarch64.tar.bz2",
        PlatformTriple(
            arch="x86_64", platform="macos", environment=None, flavor=""
        ): "https://downloads.python.org/pypy/pypy3.10-v7.3.12-macos_x86_64.tar.bz2",
        PlatformTriple(
            arch="aarch64", platform="macos", environment=None, flavor=""
        ): "https://downloads.python.org/pypy/pypy3.10-v7.3.12-macos_arm64.tar.bz2",
        PlatformTriple(
            arch="x86_64", platform="windows", environment=None, flavor=""
        ): "https://downloads.python.org/pypy/pypy3.10-v7.3.12-win64.zip",
    },
    PythonVersion(3, 9, 16): {
        PlatformTriple(
            arch="x86_64", platform="linux", environment="gnu", flavor=""
        ): "https://downloads.python.org/pypy/pypy3.9-v7.3.11-linux64.tar.bz2",
        PlatformTriple(
            arch="aarch64", platform="linux", environment="gnu", flavor=""
        ): "https://downloads.python.org/pypy/pypy3.9-v7.3.11-aarch64.tar.bz2",
        PlatformTriple(
            arch="x86_64", platform="macos", environment=None, flavor=""
        ): "https://downloads.python.org/pypy/pypy3.9-v7.3.11-macos_x86_64.tar.bz2",
        PlatformTriple(
            arch="aarch64", platform="macos", environment=None, flavor=""
        ): "https://downloads.python.org/pypy/pypy3.9-v7.3.11-macos_arm64.tar.bz2",
        PlatformTriple(
            arch="x86_64", platform="windows", environment=None, flavor=""
        ): "https://downloads.python.org/pypy/pypy3.9-v7.3.11-win64.zip",
    },
    PythonVersion(3, 8, 16): {
        PlatformTriple(
            arch="x86_64", platform="linux", environment="gnu", flavor=""
        ): "https://downloads.python.org/pypy/pypy3.8-v7.3.11-linux64.tar.bz2",
        PlatformTriple(
            arch="aarch64", platform="linux", environment="gnu", flavor=""
        ): "https://downloads.python.org/pypy/pypy3.8-v7.3.11-aarch64.tar.bz2",
        PlatformTriple(
            arch="x86_64", platform="macos", environment=None, flavor=""
        ): "https://downloads.python.org/pypy/pypy3.8-v7.3.11-macos_x86_64.tar.bz2",
        PlatformTriple(
            arch="aarch64", platform="macos", environment=None, flavor=""
        ): "https://downloads.python.org/pypy/pypy3.8-v7.3.11-macos_arm64.tar.bz2",
        PlatformTriple(
            arch="x86_64", platform="windows", environment=None, flavor=""
        ): "https://downloads.python.org/pypy/pypy3.8-v7.3.11-win64.zip",
    },
    PythonVersion(3, 7, 13): {
        PlatformTriple(
            arch="x86_64", platform="linux", environment="gnu", flavor=""
        ): "https://downloads.python.org/pypy/pypy3.7-v7.3.9-linux64.tar.bz2",
        PlatformTriple(
            arch="aarch64", platform="linux", environment="gnu", flavor=""
        ): "https://downloads.python.org/pypy/pypy3.7-v7.3.9-aarch64.tar.bz2",
        PlatformTriple(
            arch="x86_64", platform="macos", environment=None, flavor=""
        ): "https://downloads.python.org/pypy/pypy3.7-v7.3.9-osx64.tar.bz2",
        PlatformTriple(
            arch="x86_64", platform="windows", environment=None, flavor=""
        ): "https://downloads.python.org/pypy/pypy3.7-v7.3.9-win64.zip",
    },
}

if __name__ == "__main__":
    main()


class Tests(unittest.TestCase):
    def test_parse_triplets(self):
        expected = {
            "aarch64-apple-darwin-lto": PlatformTriple("aarch64", "macos", None, "lto"),
            "aarch64-unknown-linux-gnu-pgo+lto": PlatformTriple(
                "aarch64", "linux", "gnu", "pgo+lto"
            ),
            # "x86_64-unknown-linux-musl-debug": PlatformTriple(
            #     "x86_64", "linux", "musl", "debug"
            # ),
            "aarch64-unknown-linux-gnu-debug-full": PlatformTriple(
                "aarch64", "linux", "gnu", "debug"
            ),
            "x86_64-unknown-linux-gnu-debug": PlatformTriple(
                "x86_64", "linux", "gnu", "debug"
            ),
            "linux64": PlatformTriple("x86_64", "linux", "gnu", ""),
            "ppc64le-unknown-linux-gnu-noopt-full": None,
            "x86_64_v3-unknown-linux-gnu-lto": None,
            "x86_64-pc-windows-msvc-shared-pgo": PlatformTriple(
                "x86_64", "windows", None, "shared-pgo"
            ),
        }

        for input, expected in expected.items():
            self.assertEqual(PlatformTriple.from_str(input), expected, input)
