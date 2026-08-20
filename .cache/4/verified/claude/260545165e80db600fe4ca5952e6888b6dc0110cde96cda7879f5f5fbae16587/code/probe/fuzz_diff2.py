#!/usr/bin/env python3
"""Differential fuzzing of the C driver vs the Rust driver, biased towards the
buffer-overread paths (buffers that are not NUL terminated, so the C code reads
the uninitialised parts of its stack frame)."""
import random
import subprocess
import sys

C = sys.argv[1] if len(sys.argv) > 1 else "c_src/build/driver"
R = sys.argv[2] if len(sys.argv) > 2 else "target/release/driver"
N = int(sys.argv[3]) if len(sys.argv) > 3 else 500
SEED = int(sys.argv[4]) if len(sys.argv) > 4 else 1

WORDS = [b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET", b"ADMIN", b"VALID", b"OK",
         b"NONE", b"EMPTY", b"abc", b"ABC", b"", b"A", b"_v1", b"_old", b"*", b"a:b"]


def buf(rnd):
    k = rnd.randrange(6)
    if k == 0:                                  # unterminated fixed length run
        n = rnd.choice(list(range(0, 40)) + [64, 100, 200, 511, 512, 1000, 1023, 1024])
        c = rnd.choice([65, 97, 66, 32, 255, 1, 127, 128])
        return bytes([c] * n)
    if k == 1:                                  # unterminated random
        n = rnd.randrange(0, 40)
        return bytes(rnd.randrange(1, 256) for _ in range(n))
    if k == 2:                                  # word, maybe unterminated
        w = bytes(rnd.choice(WORDS))
        return w if rnd.random() < 0.5 else w + b"\0"
    if k == 3:                                  # word + junk tail
        w = bytes(rnd.choice(WORDS))
        return w + bytes(rnd.randrange(0, 256) for _ in range(rnd.randrange(0, 6)))
    if k == 4:                                  # terminated random
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


def run(path, data):
    try:
        p = subprocess.run([path], input=data, capture_output=True, text=True, timeout=15)
        return p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        return "timeout", "", ""


def main():
    rnd = random.Random(SEED)
    bad = 0
    for i in range(N):
        data = make_case(rnd)
        c = run(C, data)
        r = run(R, data)
        if c[0] != r[0] or c[1] != r[1] or c[2] != r[2]:
            bad += 1
            if bad <= 12:
                print(f"MISMATCH #{bad}: input={data.strip()[:200]}")
                print(f"   C: rc={c[0]} out={c[1]!r} err={c[2][:70]!r}")
                print(f"   R: rc={r[0]} out={r[1]!r} err={r[2][:70]!r}")
    print(f"total={N} mismatches={bad}")


if __name__ == "__main__":
    main()
