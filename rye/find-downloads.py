import re
import requests
from urllib.parse import unquote


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
    "static-noopt",
    "gnu-pgo+lto",
    "gnu-lto",
    "gnu-pgo",
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
    "linux64": "x86_64-unknown-linux",
    "windows-amd64": "x86_64-pc-windows",
    "windows-x86": "i686-pc-windows",
    "linux64-musl": "x86_64-unknown-linux",
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

_filename_re = re.compile(
    r"""(?x)
    ^
        cpython-(?P<ver>\d+\.\d+\.\d+?)
        (?:\+\d+)?
        -(?P<triple>.*?)
        (?:-[\dT]+)?\.tar\.(?:gz|zst)
    $
"""
)
_suffix_re = re.compile(
    r"""(?x)^(.*?)-(%s)$"""
    % (
        "|".join(
            map(
                re.escape,
                sorted(FLAVOR_PREFERENCES + HIDDEN_FLAVORS, key=len, reverse=True),
            )
        )
    )
)


def parse_filename(filename):
    match = _filename_re.match(filename)
    if match is None:
        return
    version, triple = match.groups()
    if triple.endswith("-full"):
        triple = triple[:-5]
    match = _suffix_re.match(triple)
    if match is not None:
        triple, suffix = match.groups()
    else:
        suffix = None
    return (version, triple, suffix)


def normalize_triple(triple):
    if "-musl" in triple or "-static" in triple:
        return
    triple = SPECIAL_TRIPLES.get(triple, triple)
    pieces = triple.split("-")
    try:
        arch = ARCH_MAPPING.get(pieces[0])
        if arch is None:
            return
        platform = PLATFORM_MAPPING.get(pieces[2])
        if platform is None:
            return
    except IndexError:
        return
    return "%s-%s" % (arch, platform)


results = {}
sess = requests.Session()

for page in range(1, 100):
    resp = sess.get("%s?page=%d" % (RELEASE_URL, page), headers=HEADERS)
    rows = resp.json()
    if not rows:
        break
    for row in rows:
        for asset in row["assets"]:
            url = asset["browser_download_url"]
            base_name = unquote(url.rsplit("/")[-1])
            # These are currently broken: https://github.com/indygreg/python-build-standalone/issues/172
            if "20230507" in base_name:
                continue
            if base_name.endswith(".sha256"):
                continue
            info = parse_filename(base_name)
            if info is None:
                continue
            py_ver, triple, flavor = info
            if "-static" in triple or (flavor and 'noopt' in flavor):
                continue
            triple = normalize_triple(triple)
            if triple is None:
                continue
            results.setdefault(py_ver, []).append((triple, flavor, url))


def _sort_key(info):
    triple, flavor, url = info
    try:
        pref = FLAVOR_PREFERENCES.index(flavor)
    except ValueError:
        pref = -1
    return tuple, pref


final_results = {}
for py_ver, choices in results.items():
    choices.sort(key=_sort_key)
    urls = {}
    for triple, flavor, url in choices:
        if triple in urls:
            continue
        urls[tuple(triple.split('-'))] = url
    final_results[tuple(map(int, py_ver.split('.')))] = urls


print("// generated code, do not edit")
print("use std::borrow::Cow;")
print("pub const CPYTHON_VERSIONS: &[(PythonVersion, &str, &str, &str)] = &[")
for py_ver, choices in sorted(
    final_results.items(), key=lambda x: x[0], reverse=True
):
    for (arch, platform), url in sorted(choices.items()):
        print('    (PythonVersion { kind: Cow::Borrowed("cpython"), major: %d, minor: %d, patch: %d, suffix: None }, "%s", "%s", "%s"),' % (py_ver + (arch, platform, url)))
print("];")
