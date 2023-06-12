import sys
import hashlib

h = hashlib.sha256()

with open(sys.argv[1], "rb") as f:
    while True:
        if not (chunk := f.read(4096)):
            break        
        h.update(chunk)

print(h.hexdigest())