#!/usr/bin/env python3
"""Sanity-check the generated vectors against the C .so via ctypes.

Catches assert() aborts / crashes before they can take down the Rust test
process, and verifies the vectors actually round-trip.
"""
import ctypes
import os
import subprocess
import sys
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "data")
SO = sys.argv[1]

CHILD = """
import ctypes, sys, zlib
so, path, rawlen = sys.argv[1], sys.argv[2], int(sys.argv[3])
lib = ctypes.CDLL(so)
lib.cp_inflate.restype = ctypes.c_int
lib.cp_inflate.argtypes = [ctypes.c_void_p, ctypes.c_int,
                           ctypes.c_void_p, ctypes.c_int]
data = open(path, 'rb').read()
inbuf = ctypes.create_string_buffer(data, len(data) + 8)
cap = rawlen + 4096
out = ctypes.create_string_buffer(cap)
r = lib.cp_inflate(ctypes.cast(inbuf, ctypes.c_void_p), len(data),
                   ctypes.cast(out, ctypes.c_void_p), cap)
sys.stdout.write('%d %s\\n' % (r, out.raw[:rawlen].hex()))
"""

bad = 0
for line in open(os.path.join(DATA, "manifest.txt")):
    name, dlen, rawlen = line.split()
    rawlen = int(rawlen)
    path = os.path.join(DATA, name + ".deflate")
    p = subprocess.run([sys.executable, "-c", CHILD, SO, path, str(rawlen)],
                       capture_output=True)
    if p.returncode != 0:
        print("CRASH %-28s rc=%d %s" % (name, p.returncode,
                                        p.stderr.decode()[:120]))
        bad += 1
        continue
    parts = p.stdout.decode().split()
    ret = int(parts[0])
    got = bytes.fromhex(parts[1]) if len(parts) > 1 else b""
    if ret != 1:
        print("RET0  %-28s" % name)
        bad += 1
        continue
    try:
        want = zlib.decompress(open(path, "rb").read(), -15)
    except Exception as e:
        print("SKIP  %-28s (python: %s)" % (name, e))
        continue
    if got != want:
        print("DIFF  %-28s len got=%d want=%d" % (name, len(got), len(want)))
        bad += 1

print("done, %d problem vectors" % bad)
