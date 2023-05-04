import os
import re
import sys
import json
from importlib.metadata import distribution, PackageNotFoundError

_package_re = re.compile("(?i)^([a-z0-9._-]+)")

result = {}


def dump_all(dist, root=False):
    rv = []
    for file in dist.files or ():
        rv.append(os.path.normpath(dist.locate_file(file)))
    result["" if root else dist.name] = rv
    req = []
    for r in dist.requires or ():
        name = _package_re.match(r)
        if name is not None:
            req.append(name.group())
    return req


root = sys.argv[1]
to_resolve = [root]
seen = set()
while to_resolve:
    try:
        d = to_resolve.pop()
        dist = distribution(d)
    except Exception:
        continue
    if dist.name in seen:
        continue
    seen.add(dist.name)
    to_resolve.extend(dump_all(dist, root=d == root))

print(json.dumps(result))
