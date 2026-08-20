#!/usr/bin/env python3
"""Differential fuzzing that separates real divergences from the C program's own
nondeterminism.

The C code reads the uninitialised part of its stack frame, and the low byte of
the loader's saved stack pointers changes with ASLR on every run, so for some
inputs the C program prints different results on different runs.  This script
runs the C program N times per input and only reports a mismatch when the C
result is *stable* and still differs from the Rust result."""
import random
import subprocess
import sys

C = sys.argv[1] if len(sys.argv) > 1 else "c_src/build/driver"
R = sys.argv[2] if len(sys.argv) > 2 else "target/release/driver"
N = int(sys.argv[3]) if len(sys.argv) > 3 else 300
SEED = int(sys.argv[4]) if len(sys.argv) > 4 else 1
REPEAT = int(sys.argv[5]) if len(sys.argv) > 5 else 5

sys.path.insert(0, "probe")
from fuzz_diff2 import make_case, run  # noqa: E402  (reuse the generator)

rnd = random.Random(SEED)
stable_bad = 0
unstable = 0
for i in range(N):
    data = make_case(rnd)
    runs = [run(C, data) for _ in range(REPEAT)]
    r = run(R, data)
    key = [(x[0], x[1]) for x in runs]
    if len(set(map(str, key))) != 1:
        unstable += 1
        continue
    c = runs[0]
    if (c[0], c[1], c[2]) != (r[0], r[1], r[2]):
        stable_bad += 1
        if stable_bad <= 12:
            print(f"STABLE MISMATCH #{stable_bad}: input={data.strip()[:200]}")
            print(f"   C: rc={c[0]} out={c[1]!r} err={c[2][:80]!r}")
            print(f"   R: rc={r[0]} out={r[1]!r} err={r[2][:80]!r}")
print(f"total={N} stable_mismatches={stable_bad} inputs_where_C_is_nondeterministic={unstable}")
