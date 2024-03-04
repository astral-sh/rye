import itertools
import sys
import time
from datetime import datetime, timezone
from typing import NamedTuple, Self

import httpx


def log(*args, **kwargs):
    print(*args, file=sys.stderr, **kwargs)


def batched(iterable, n):
    "Batch data into tuples of length n. The last batch may be shorter."
    # batched('ABCDEFG', 3) --> ABC DEF G
    if n < 1:
        raise ValueError("n must be at least one")
    it = iter(iterable)
    while batch := tuple(itertools.islice(it, n)):
        yield batch


class PlatformTriple(NamedTuple):
    arch: str
    platform: str
    environment: str | None
    flavor: str | None


class Version(NamedTuple):
    major: int
    minor: int
    patch: int

    @classmethod
    def from_str(cls, version: str) -> Self:
        major, minor, patch = version.split(".", 3)
        return cls(int(major), int(minor), int(patch))

    def __str__(self) -> str:
        return f"{self.major}.{self.minor}.{self.patch}"

    def __neg__(self) -> Self:
        return Version(-self.major, -self.minor, -self.patch)


async def fetch(client: httpx.AsyncClient, url: str) -> httpx.Response:
    """Fetch from GitHub API with rate limit awareness."""
    resp = await client.get(url, timeout=15)
    if (
        resp.status_code in [403, 429]
        and resp.headers.get("x-ratelimit-remaining") == "0"
    ):
        # See https://docs.github.com/en/rest/using-the-rest-api/troubleshooting-the-rest-api?apiVersion=2022-11-28
        if (retry_after := resp.headers.get("retry-after")) is not None:
            log(f"Got retry-after header, retry in {retry_after} seconds.")
            time.sleep(int(retry_after))

            return await fetch(client, url)

        if (retry_at := resp.headers.get("x-ratelimit-reset")) is not None:
            utc = datetime.now(timezone.utc).timestamp()
            retry_after = max(int(retry_at) - int(utc), 0)

            log(f"Got x-ratelimit-reset header, retry in {retry_after} seconds.")
            time.sleep(retry_after)

            return await fetch(client, url)

        log("Got rate limited but no information how long, wait for 2 minutes.")
        time.sleep(60 * 2)
        return await fetch(client, url)

    resp.raise_for_status()
    return resp
