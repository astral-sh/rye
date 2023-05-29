import sys
import hashlib

h = hashlib.sha256()

with open(sys.argv[1], "rb") as f:
    while True:
        chunk = f.read(4096)
        if not chunk:
            break
        h.update(chunk)

print(h.hexdigest())