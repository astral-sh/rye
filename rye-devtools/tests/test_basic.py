import pytest
from rye_devtools.common import batched
from rye_devtools.find_downloads import CPythonFinder, PlatformTriple


def test_batched():
    assert list(batched("ABCDEFG", 3)) == [tuple("ABC"), tuple("DEF"), tuple("G")]


@pytest.mark.parametrize(
    "input, expected",
    [
        ("aarch64-apple-darwin-lto", PlatformTriple("aarch64", "macos", None, "lto")),
        (
            "aarch64-unknown-linux-gnu-pgo+lto",
            PlatformTriple("aarch64", "linux", "gnu", "pgo+lto"),
        ),
        # (
        #     "x86_64-unknown-linux-musl-debug",
        #     PlatformTriple("x86_64", "linux", "musl", "debug"),
        # ),
        (
            "aarch64-unknown-linux-gnu-debug-full",
            PlatformTriple("aarch64", "linux", "gnu", "debug"),
        ),
        (
            "x86_64-unknown-linux-gnu-debug",
            PlatformTriple("x86_64", "linux", "gnu", "debug"),
        ),
        ("linux64", PlatformTriple("x86_64", "linux", "gnu", None)),
        ("ppc64le-unknown-linux-gnu-noopt-full", None),
        ("x86_64_v3-unknown-linux-gnu-lto", None),
        (
            "x86_64-pc-windows-msvc-shared-pgo",
            PlatformTriple("x86_64", "windows", None, "shared-pgo"),
        ),
    ],
)
def test_parse_triplets(input, expected):
    assert CPythonFinder.parse_triple(input) == expected
