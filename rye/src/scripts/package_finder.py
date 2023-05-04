import sys
import json
from unearth.finder import PackageFinder
from packaging.version import Version

py_ver = sys.argv[1]
package = sys.argv[2]
pre = len(sys.argv) > 3 and sys.argv[3] == "--pre"

finder = PackageFinder(
    index_urls=["https://pypi.org/simple/"],
)
if py_ver:
    finder.target_python.py_ver = tuple(map(int, py_ver.split(".")))
choices = iter(finder.find_matches(package))
if not pre:
    choices = (m for m in choices if not Version(m.version).is_prerelease)

print(json.dumps([x.as_json() for x in choices]))
