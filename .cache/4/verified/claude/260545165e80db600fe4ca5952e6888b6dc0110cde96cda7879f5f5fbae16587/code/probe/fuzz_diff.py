#!/usr/bin/env python3
"""Differential fuzzing of the C driver vs the Rust driver (stdin -> stdout)."""
import random
import subprocess
import sys

C = sys.argv[1] if len(sys.argv) > 1 else "c_src/build/driver"
R = sys.argv[2] if len(sys.argv) > 2 else "target/release/driver"
N = int(sys.argv[3]) if len(sys.argv) > 3 else 500
SEED = int(sys.argv[4]) if len(sys.argv) > 4 else 1234

WORDS = [b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET", b"ADMIN", b"VALID", b"OK",
         b"NONE", b"EMPTY", b"abc", b"ABC", b"", b"_v1", b"_v2", b"_old", b"_new",
         b"_tmp", b"*", b"a:b", b"x|y"]


def rand_bytes(rnd):
    kind = rnd.randrange(6)
    if kind == 0:
        w = rnd.choice(WORDS)
        b = bytearray(w)
        if rnd.random() < 0.6:
            b.append(0)
        if rnd.random() < 0.3:
            b += bytes(rnd.choice(WORDS))
            if rnd.random() < 0.5:
                b.append(0)
        return bytes(b)
    if kind == 1:
        n = rnd.randrange(0, 12)
        return bytes(rnd.randrange(1, 128) for _ in range(n))
    if kind == 2:
        n = rnd.randrange(0, 12)
        return bytes(rnd.randrange(0, 256) for _ in range(n))
    if kind == 3:
        w = rnd.choice(WORDS)
        return bytes(w) + b"\0" + bytes(rnd.randrange(0, 256) for _ in range(rnd.randrange(0, 5)))
    if kind == 4:
        n = rnd.choice([0, 1, 2, 1023, 1024])
        return bytes(rnd.choice([0, 65, 97, 32, 255]) for _ in range(n))
    n = rnd.randrange(0, 70)
    return bytes(rnd.choice([65, 97, 0, 42, 58, 124, 95]) for _ in range(n))


def make_case(rnd):
    op = rnd.choice([0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 5, -1, 99, -3])
    flags = rnd.choice([0, 1, 2, 3, 0, 1, 2, 3, 4, 0xFFFFFFFF])
    inp = rand_bytes(rnd)
    ref = rand_bytes(rnd)
    toks = [op, flags, len(inp)] + list(inp) + [len(ref)] + list(ref)
    return " ".join(str(t) for t in toks) + "\n"


def run(path, data):
    try:
        p = subprocess.run([path], input=data, capture_output=True, text=True, timeout=10)
        return p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        return "timeout", "", ""


rnd = random.Random(SEED)
bad = 0
for i in range(N):
    data = make_case(rnd)
    c = run(C, data)
    r = run(R, data)
    if c[0] != r[0] or c[1] != r[1]:
        bad += 1
        if bad <= 15:
            print(f"MISMATCH #{bad}: input={data.strip()[:160]}")
            print(f"   C: rc={c[0]} out={c[1]!r} err={c[2][:60]!r}")
            print(f"   R: rc={r[0]} out={r[1]!r} err={r[2][:60]!r}")
print(f"total={N} mismatches={bad}")
