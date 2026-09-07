#!/usr/bin/env python3
"""Mutate raw DEFLATE streams (and craft random ones) for cp_inflate fuzzing."""
import os, random, sys, glob, zlib

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, "corpus", "inflate")
DST = os.path.join(HERE, "fuzz", "inf")
os.makedirs(DST, exist_ok=True)
for f in glob.glob(os.path.join(DST, "*")):
    os.remove(f)

seeds = [open(p, "rb").read() for p in sorted(glob.glob(os.path.join(SRC, "*")))]
seeds = [s for s in seeds if s]
rnd = random.Random(int(sys.argv[1]) if len(sys.argv) > 1 else 99)
n = int(sys.argv[2]) if len(sys.argv) > 2 else 3000

for i in range(n):
    if rnd.randrange(4) == 0:
        # completely random stream
        ln = rnd.randrange(1, 96)
        data = bytearray(rnd.randrange(256) for _ in range(ln))
    else:
        data = bytearray(seeds[rnd.randrange(len(seeds))])
        k = rnd.randrange(4)
        if k == 0:
            for _ in range(rnd.randrange(1, 4)):
                data[rnd.randrange(len(data))] ^= 1 << rnd.randrange(8)
        elif k == 1:
            for _ in range(rnd.randrange(1, 6)):
                data[rnd.randrange(len(data))] = rnd.randrange(256)
        elif k == 2:
            data = data[:rnd.randrange(1, len(data) + 1)]
        else:
            a = rnd.randrange(len(data))
            data[a:a] = bytes(rnd.randrange(256) for _ in range(rnd.randrange(1, 8)))
    open(os.path.join(DST, "i%05d" % i), "wb").write(bytes(data))
print("inflate fuzz cases:", n)
