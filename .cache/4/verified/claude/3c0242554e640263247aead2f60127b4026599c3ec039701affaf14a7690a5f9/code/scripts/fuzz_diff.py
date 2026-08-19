#!/usr/bin/env python3
"""Randomized end-to-end differential fuzz of the C and Rust executables.

Feeds identical stdin bytes to c_src/build/driver and target/release/driver and
compares stdout, stderr and the exit status byte for byte. Seeded, so failures
are reproducible.
"""
import os
import random
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
C_EXE = os.path.join(ROOT, "c_src", "build", "driver")
R_EXE = os.path.join(ROOT, "target", "release", "driver")

SEED = 0x5EED_1234
CASES = 2000

WS = [b" ", b"\t", b"\n", b"\v", b"\f", b"\r"]
JUNK = list(b"0123456789+-abcxyzXYZ.,;:*/\\'\"%$#@!()[]{}") + [0, 0x7F, 0x80, 0xFF]


def gen(rng):
    kind = rng.randrange(8)
    out = bytearray()
    for _ in range(rng.randrange(4)):
        out += rng.choice(WS)
    if kind == 0:                                   # plain int32
        out += str(rng.randint(-(2**31), 2**31 - 1)).encode()
    elif kind == 1:                                 # int64-ish / huge
        out += str(rng.randint(-(2**80), 2**80)).encode()
    elif kind == 2:                                 # boundary values
        out += rng.choice([
            b"2147483647", b"-2147483648", b"2147483648", b"-2147483649",
            b"4294967295", b"4294967296", b"9223372036854775807",
            b"9223372036854775808", b"-9223372036854775808",
            b"-9223372036854775809", b"0", b"-0", b"+0",
        ])
    elif kind == 3:                                 # leading zeros
        out += rng.choice([b"", b"-", b"+"]) + b"0" * rng.randrange(30)
        out += str(rng.randrange(1000)).encode()
    elif kind == 4:                                 # random junk
        out += bytes(rng.choice(JUNK) for _ in range(rng.randrange(20)))
    elif kind == 5:                                 # number + trailing junk
        out += str(rng.randint(-9999, 9999)).encode()
        out += bytes(rng.choice(JUNK) for _ in range(rng.randrange(8)))
    elif kind == 6:                                 # several tokens
        out += b" ".join(str(rng.randint(-9999, 9999)).encode()
                         for _ in range(rng.randrange(1, 5)))
    else:                                           # long whitespace prefix
        out += rng.choice(WS) * rng.randrange(4000, 9000)
        out += str(rng.randint(-(2**31), 2**31 - 1)).encode()
    for _ in range(rng.randrange(3)):
        out += rng.choice(WS)
    return bytes(out)


def run(exe, data):
    p = subprocess.Popen(exe, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                         stderr=subprocess.PIPE)
    try:
        out, err = p.communicate(data, timeout=60)
    except subprocess.TimeoutExpired:
        p.kill()
        out, err = p.communicate()
        return ("TIMEOUT", out, err)
    return (p.returncode, out, err)


def main():
    for exe in (C_EXE, R_EXE):
        if not os.path.exists(exe):
            print(f"missing {exe}", file=sys.stderr)
            return 1
    rng = random.Random(SEED)
    for i in range(CASES):
        data = gen(rng)
        a = run(C_EXE, data)
        b = run(R_EXE, data)
        if a != b:
            print(f"DIVERGENCE at case {i}\ninput={data[:200]!r} (len {len(data)})"
                  f"\nC   ={a}\nRust={b}", file=sys.stderr)
            return 1
    print(f"{CASES} randomized cases: identical")
    return 0


if __name__ == "__main__":
    sys.exit(main())
