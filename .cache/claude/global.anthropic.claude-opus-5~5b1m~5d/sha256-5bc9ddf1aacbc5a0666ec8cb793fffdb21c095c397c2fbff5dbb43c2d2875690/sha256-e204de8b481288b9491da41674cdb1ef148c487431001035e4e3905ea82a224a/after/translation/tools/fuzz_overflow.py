#!/usr/bin/env python3
"""Differential fuzz biased towards distances that overflow `int`, run from
`translation/`:

    python3 tools/fuzz_overflow.py <seed> <rounds>

These are the sessions that make `find_shortest_path` overrun its `path` array
into the Dijkstra state.  As in tools/fuzz.py, a difference is only reported when
the C program agrees with itself across repeated runs.
"""
import random
import subprocess
import sys

C = "../c_src/build/driver"
R = "target/release/driver"


def run(binary, data):
    p = subprocess.run([binary], input=data, stdout=subprocess.PIPE,
                       stderr=subprocess.PIPE, timeout=30)
    return p.stdout, p.stderr, p.returncode


def gen(rng):
    cities = ["C%d" % k for k in range(1, 8)]
    big = ["2147483647", "2147483646", "2000000000", "1500000000", "1073741824",
           "1", "2", "0"]
    out = []
    for c in cities:
        out += ["1", c]
    for _ in range(rng.randint(4, 14)):
        a, b = rng.choice(cities), rng.choice(cities)
        out += ["2", a, b, rng.choice(big)]
    for _ in range(rng.randint(1, 4)):
        out += ["5", rng.choice(cities), rng.choice(cities)]
    if rng.random() < 0.5:
        out += ["3"]
    out += ["8"]
    return ("\n".join(out) + "\n").encode()


def main():
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else 1
    rounds = int(sys.argv[2]) if len(sys.argv) > 2 else 200
    rng = random.Random(seed)
    bad = 0
    skipped = 0
    for _ in range(rounds):
        data = gen(rng)
        c1 = run(C, data)
        r1 = run(R, data)
        if c1 == r1:
            continue
        if any(run(C, data) != c1 for _ in range(3)):
            skipped += 1
            continue
        bad += 1
        print("=== MISMATCH (C is deterministic here) ===")
        print("input:", data)
        for name, a, b in (("stdout", c1[0], r1[0]), ("stderr", c1[1], r1[1]),
                           ("status", c1[2], r1[2])):
            if a != b:
                print(f"  {name}: C={a[:400]!r}\n         R={b[:400]!r}")
        if bad >= 3:
            break
    print(f"rounds={rounds} mismatches={bad} nondeterministic_skipped={skipped}")


main()
