import json
import os
import sys

version = sys.argv[1]
base = sys.argv[2]

checksums = {}

for folder in os.listdir(base):
    for filename in os.listdir(os.path.join(base, folder)):
        if filename.endswith(".sha256"):
            with open(os.path.join(base, folder, filename)) as f:
                sha256 = f.read().strip()
            checksums[filename[:-7]] = sha256

print(
    json.dumps(
        {
            "version": version,
            "checksums": checksums,
        },
        indent=2,
    )
)
