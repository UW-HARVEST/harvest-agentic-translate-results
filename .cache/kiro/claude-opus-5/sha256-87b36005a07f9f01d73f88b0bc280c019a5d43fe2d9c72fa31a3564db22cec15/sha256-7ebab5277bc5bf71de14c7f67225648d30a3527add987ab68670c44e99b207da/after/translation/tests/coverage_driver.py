#!/usr/bin/env python3
"""Drive the instrumented C library over every vector (plus convert_pix) so
gcov can report which lines of lib.c the test corpus never reaches."""
import ctypes
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "data")
lib = ctypes.CDLL(sys.argv[1])
lib.cp_inflate.restype = ctypes.c_int
lib.cp_inflate.argtypes = [ctypes.c_void_p, ctypes.c_int,
                           ctypes.c_void_p, ctypes.c_int]
lib.convert_pix.restype = None
lib.convert_pix.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_int,
                            ctypes.c_void_p, ctypes.c_void_p]

skip = set(os.environ.get("SKIP", "").split(","))
for line in open(os.path.join(DATA, "manifest.txt")):
    name, _dlen, rawlen = line.split()
    if name in skip:
        continue
    rawlen = int(rawlen)
    data = open(os.path.join(DATA, name + ".deflate"), "rb").read()
    cap = rawlen + 65600
    for align in range(4):
        storage = ctypes.create_string_buffer(len(data) + 32)
        base = ctypes.addressof(storage)
        off = 8
        while (base + off) % 4 != align:
            off += 1
        ctypes.memmove(base + off, data, len(data))
        out = ctypes.create_string_buffer(cap)
        for ob in (cap, rawlen, rawlen // 2, 1, 0):
            lib.cp_inflate(base + off, len(data), ctypes.addressof(out), ob)

for bpp in (0, 1, 2, 3, 4, 5, -1):
    for (w, h) in ((0, 0), (1, 1), (5, 3), (17, 9)):
        per = 1 + max(w, 0) * max(bpp, 0)
        src = ctypes.create_string_buffer(per * max(h, 0) + 64)
        dst = ctypes.create_string_buffer(max(w, 0) * max(h, 0) * 4 + 32)
        lib.convert_pix(bpp, w, h, ctypes.addressof(src), ctypes.addressof(dst))
print("driver done")
