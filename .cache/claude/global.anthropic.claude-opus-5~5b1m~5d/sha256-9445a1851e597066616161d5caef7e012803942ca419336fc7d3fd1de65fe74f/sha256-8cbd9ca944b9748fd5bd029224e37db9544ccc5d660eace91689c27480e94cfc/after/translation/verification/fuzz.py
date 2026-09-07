#!/usr/bin/env python3
"""Byte-mutate valid PNGs to produce a fuzz corpus."""
import os, random, sys, glob

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, "corpus", "png")
DST = os.path.join(HERE, "fuzz", "png")
os.makedirs(DST, exist_ok=True)
for f in glob.glob(os.path.join(DST, "*")):
    os.remove(f)

seeds = sorted(glob.glob(os.path.join(SRC, "ok_*")))
rnd = random.Random(int(sys.argv[1]) if len(sys.argv) > 1 else 1234)
n = int(sys.argv[2]) if len(sys.argv) > 2 else 2000
made = 0
while made < n:
    src = seeds[rnd.randrange(len(seeds))]
    data = bytearray(open(src, "rb").read())
    if not data:
        continue
    kind = rnd.randrange(5)
    if kind == 0:                       # single byte flip
        for _ in range(rnd.randrange(1, 4)):
            data[rnd.randrange(len(data))] ^= 1 << rnd.randrange(8)
    elif kind == 1:                     # random byte overwrite
        for _ in range(rnd.randrange(1, 8)):
            data[rnd.randrange(len(data))] = rnd.randrange(256)
    elif kind == 2:                     # truncate
        data = data[:rnd.randrange(1, len(data))]
    elif kind == 3:                     # splice/duplicate a slice
        a = rnd.randrange(len(data))
        b = min(len(data), a + rnd.randrange(1, 64))
        data[a:a] = data[a:b]
    else:                               # zero or fill a run
        a = rnd.randrange(len(data))
        b = min(len(data), a + rnd.randrange(1, 32))
        v = rnd.choice([0, 255, rnd.randrange(256)])
        for i in range(a, b):
            data[i] = v
    open(os.path.join(DST, "m%05d_%s" % (made, os.path.basename(src))), "wb").write(bytes(data))
    made += 1
print("fuzz cases:", made)
