"""This script is used to generate rye/src/sources/generated/uv_downloads.inc.

It finds the latest UV releases and generates rust code that can be included
into rye at build time.
"""

import asyncio
import os
import re
import sys
from dataclasses import dataclass
from typing import AsyncIterator

import httpx

from .common import PlatformTriple, Version, fetch, log


@dataclass
class UvDownload:
    triple: PlatformTriple
    version: Version
    url: str
    sha256: str


class UvDownloads:
    client: httpx.Client

    RELEASE_URL = "https://api.github.com/repos/astral-sh/uv/releases"

    ARCH = {
        "x86_64": "x86_64",
        "i686": "i686",
        "aarch64": "aarch64",
    }

    PLATFORM_ENV = {
        "unknown-linux-gnu": ("linux", "gnu"),
        "unknown-linux-musl": ("linux", "musl"),
        "apple-darwin": ("macos", None),
        "pc-windows-msvc": ("windows", None),
    }

    RE = re.compile(r"uv-(?P<arch>[^\-]+)-(?P<plat_env>.+)(\.tar\.gz|\.zip)$")

    def __init__(self, client: httpx.Client) -> None:
        self.client = client

    async def most_recent_downloads(
        self, pages: int = 100
    ) -> AsyncIterator[UvDownload]:
        highest_version = None
        for page in range(1, pages):
            log(f"fetching page {page}")
            resp = await fetch(self.client, "%s?page=%d" % (self.RELEASE_URL, page))
            rows = resp.json()
            if not rows:
                break
            for row in rows:
                version = Version.from_str(row["tag_name"])
                if highest_version is None or highest_version < version:
                    for asset in row["assets"]:
                        url = asset["browser_download_url"]
                        if (triple := self.parse_triple(url)) is not None:
                            sha_resp = await fetch(self.client, url + ".sha256")
                            sha256 = sha_resp.text.split(" ")[0].strip()
                            yield UvDownload(
                                triple=triple,
                                version=version,
                                url=url,
                                sha256=sha256,
                            )
                    highest_version = version

    @classmethod
    def parse_triple(cls, url: str) -> PlatformTriple | None:
        if (m := re.search(cls.RE, url)) is not None:
            arch_str = m.group("arch")
            plat_env_str = m.group("plat_env")
            if arch_str in cls.ARCH and plat_env_str in cls.PLATFORM_ENV:
                arch = cls.ARCH[arch_str]
                plat, env = cls.PLATFORM_ENV[plat_env_str]
                return PlatformTriple(
                    arch=arch, platform=plat, environment=env, flavor=None
                )

        return None


def render(downloads: list[UvDownload]):
    print("// Generated by rye-devtools. DO NOT EDIT.")
    print(
        "// To regenerate, run `rye run uv-downloads > rye/src/sources/generated/uv_downloads.inc` from the root of the repository."
    )
    print("use std::borrow::Cow;")
    print("pub const UV_DOWNLOADS: &[UvDownload] = &[")

    for download in downloads:
        triple = download.triple
        version = download.version
        sha = download.sha256
        url = download.url
        env = (
            f'Some(Cow::Borrowed("{triple.environment}"))'
            if triple.environment
            else "None"
        )
        print(
            f'    UvDownload {{arch: Cow::Borrowed("{triple.arch}"), os: Cow::Borrowed("{triple.platform}"), environment: {env}, major: {version.major}, minor: {version.minor}, patch: {version.patch}, suffix: None, url: Cow::Borrowed("{url}"), sha256: Cow::Borrowed("{sha}") }},'
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

    downloads = []

    log("Fetching all uv downloads.")
    async with httpx.AsyncClient(follow_redirects=True, headers=headers) as client:
        finder = UvDownloads(client)
        downloads = [download async for download in finder.most_recent_downloads()]
        log("Generating code.")
        render(downloads)


def main():
    asyncio.run(async_main())


if __name__ == "__main__":
    main()
