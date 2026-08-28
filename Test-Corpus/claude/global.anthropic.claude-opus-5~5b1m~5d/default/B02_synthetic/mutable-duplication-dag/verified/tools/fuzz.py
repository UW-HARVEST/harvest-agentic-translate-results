#!/usr/bin/env python3
"""Random differential fuzz, run from `translation/`:

    python3 tools/fuzz.py <seed> <rounds>

Random differential fuzz: only report a mismatch when the C program agrees
with itself across repeated runs (otherwise the input is in the known
ASLR-dependent class and no translation could match it)."""
import random
import subprocess
import sys

C = "../c_src/build/driver"
R = "target/release/driver"


def run(binary, data):
    p = subprocess.run([binary], input=data, stdout=subprocess.PIPE,
                       stderr=subprocess.PIPE, timeout=20)
    return p.stdout, p.stderr, p.returncode


def gen(rng, delete_heavy=False):
    cities = ["A", "B", "C", "D", "", "New York", "x" * 70, "8"]
    if delete_heavy:
        cities = ["N%d" % k for k in range(1, 15)] + ["H"]
    dists = ["0", "1", "5", "-1", "2147483647", "2000000000", "abc", "",
             "4294967301", "99999999999999999999"]
    out = []
    rounds = rng.randint(20, 90) if delete_heavy else rng.randint(1, 40)
    for _ in range(rounds):
        if delete_heavy:
            c = rng.choice([1, 1, 1, 1, 2, 2, 3, 4, 5, 6, 6, 7, 7, 7, 7, 8])
        else:
            c = rng.choice([1, 1, 1, 2, 2, 3, 4, 5, 5, 6, 6, 7, 7, 7, 9, 0, 8])
        if c == 1:
            out += ["1", rng.choice(cities)]
        elif c == 2:
            out += ["2", rng.choice(cities), rng.choice(cities), rng.choice(dists)]
        elif c in (3, 9, 0):
            out += [str(c)]
        elif c in (4, 6, 7):
            out += [str(c), rng.choice(cities)]
        elif c == 5:
            out += ["5", rng.choice(cities), rng.choice(cities)]
        elif c == 8:
            out += ["8"]
            break
    return ("\n".join(out) + "\n").encode()


def main():
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else 1
    rounds = int(sys.argv[2]) if len(sys.argv) > 2 else 300
    rng = random.Random(seed)
    bad = 0
    skipped = 0
    for i in range(rounds):
        data = gen(rng, delete_heavy=(i % 2 == 1))
        c1 = run(C, data)
        r1 = run(R, data)
        if c1 == r1:
            continue
        # is the C program self-consistent for this input?
        if any(run(C, data) != c1 for _ in range(3)):
            skipped += 1
            continue
        bad += 1
        print("=== MISMATCH (C is deterministic here) ===")
        print("input:", data)
        for name, a, b in (("stdout", c1[0], r1[0]), ("stderr", c1[1], r1[1]),
                           ("status", c1[2], r1[2])):
            if a != b:
                print(f"  {name}: C={a!r}\n         R={b!r}")
        if bad >= 5:
            break
    print(f"rounds={rounds} mismatches={bad} nondeterministic_skipped={skipped}")


main()
