#!/usr/bin/env python3
"""Differential fuzzing with a *controlled* stack frame: the C driver runs under
probe/inject_frame, which overwrites the uninitialised part of its `main` frame
with the same snapshot the Rust translation uses.  Any difference is therefore a
real difference in the translated logic (and not the environment dependent
left-overs of the dynamic loader)."""
import os
import random
import re
import subprocess
import sys

C = sys.argv[1] if len(sys.argv) > 1 else "c_src/build/driver"
R = sys.argv[2] if len(sys.argv) > 2 else "target/release/driver"
N = int(sys.argv[3]) if len(sys.argv) > 3 else 500
SEED = int(sys.argv[4]) if len(sys.argv) > 4 else 1
TMP = os.environ.get("TMPDIR", "/tmp")

bp = subprocess.run(["nm", C], capture_output=True, text=True).stdout
BP = [l.split()[0] for l in bp.splitlines() if l.endswith(" process_strings")][0]

src = open("src/frame_junk.rs").read()
table = bytes(int(x, 16) for x in re.findall(r"0x([0-9a-f]{2}),", src))
assert len(table) == 6144
JUNK = os.path.join(TMP, "junk2096_fuzz.bin")
open(JUNK, "wb").write(table[:2096])
IN = os.path.join(TMP, "inject_stdin.txt")

WORDS = [b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET", b"ADMIN", b"VALID", b"OK",
         b"NONE", b"EMPTY", b"abc", b"ABC", b"", b"A", b"_v1", b"_old", b"*", b"a:b"]


def buf(rnd):
    k = rnd.randrange(6)
    if k == 0:
        n = rnd.choice(list(range(0, 40)) + [64, 100, 200, 511, 512, 1000, 1023, 1024])
        c = rnd.choice([65, 97, 66, 32, 255, 1, 127, 128])
        return bytes([c] * n)
    if k == 1:
        n = rnd.randrange(0, 40)
        return bytes(rnd.randrange(1, 256) for _ in range(n))
    if k == 2:
        w = bytes(rnd.choice(WORDS))
        return w if rnd.random() < 0.5 else w + b"\0"
    if k == 3:
        w = bytes(rnd.choice(WORDS))
        return w + bytes(rnd.randrange(0, 256) for _ in range(rnd.randrange(0, 6)))
    if k == 4:
        n = rnd.randrange(0, 20)
        return bytes(rnd.randrange(1, 256) for _ in range(n)) + b"\0"
    n = rnd.randrange(0, 30)
    return bytes(rnd.choice([65, 97, 0, 42, 58, 124, 95, 32]) for _ in range(n))


def make_case(rnd):
    op = rnd.choice([0, 1, 2, 3, 4] * 4 + [5, -1, 7, 2147483647, -2147483648])
    flags = rnd.choice([0, 1, 2, 3] * 3 + [4, 0xFFFFFFFF, 0x80000001])
    inp = buf(rnd)
    ref = buf(rnd)
    toks = [op, flags, len(inp)] + list(inp) + [len(ref)] + list(ref)
    return " ".join(str(t) for t in toks) + "\n"


def run_c(data):
    open(IN, "w").write(data)
    try:
        p = subprocess.run(["./probe/inject_frame", C, BP, JUNK, IN],
                           capture_output=True, text=True, timeout=30)
        return p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        return "timeout", "", ""


def run_r(data):
    try:
        p = subprocess.run([R], input=data, capture_output=True, text=True, timeout=30)
        rc = p.returncode
        if rc < 0:
            rc = 128 - rc
        return rc, p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        return "timeout", "", ""


rnd = random.Random(SEED)
bad = 0
for i in range(N):
    data = make_case(rnd)
    c = run_c(data)
    r = run_r(data)
    if c != r:
        bad += 1
        if bad <= 12:
            print(f"MISMATCH #{bad}: input={data.strip()[:200]}")
            print(f"   C: {c[0]} out={c[1]!r} err={c[2][:80]!r}")
            print(f"   R: {r[0]} out={r[1]!r} err={r[2][:80]!r}")
print(f"total={N} mismatches={bad}")
