#!/usr/bin/env python3
"""Differential fuzz aimed at the heap: graphs big enough for long paths, with
deletes that free chunks the path array can then be handed."""
import random, subprocess, sys, os

ROOT = os.path.dirname(os.path.abspath(__file__))
PRE = ["setarch", "-R"] if subprocess.run(
    ["setarch", "-R", "/bin/true"], capture_output=True).returncode == 0 else []
C = PRE + [os.path.join(ROOT, "c_src/build/driver")]
R = PRE + [os.path.join(ROOT, "translation/target/release/driver")]


def gen(rng):
    n = rng.randint(20, 40)
    names = [b"N%02d" % i for i in range(n)]
    out = []
    for x in names:
        out += [b"1", x]
    # A chain, plus a few shortcuts so path lengths vary.
    for a, b in zip(names, names[1:]):
        out += [b"2", a, b, b"1"]
    for _ in range(rng.randint(0, 4)):
        i = rng.randrange(n)
        j = rng.randrange(n)
        out += [b"2", names[i], names[j], str(rng.randint(0, 5)).encode()]
    # Interleave deletes, copies, path queries and prints in random order.
    tail = []
    for _ in range(rng.randint(1, 12)):
        k = rng.randrange(6)
        victim = rng.choice(names)
        if k == 0:
            tail += [b"7", victim]
        elif k == 1:
            tail += [b"6", victim]
        elif k == 2:
            tail += [b"5", rng.choice(names), rng.choice(names)]
        elif k == 3:
            tail += [b"3"]
        elif k == 4:
            tail += [b"4", victim]
        else:
            tail += [b"1", b"X%d" % rng.randrange(50)]
    out += tail + [b"8"]
    return b"\n".join(out) + b"\n"


def run(cmd, data):
    p = subprocess.run(cmd, input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return p.stdout, p.stderr, p.returncode


def main():
    iterations = int(sys.argv[1]) if len(sys.argv) > 1 else 200
    seed0 = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    bad = 0
    for i in range(iterations):
        data = gen(random.Random(seed0 + i))
        c = run(C, data)
        r = run(R, data)
        if c != r:
            bad += 1
            path = f"/tmp/heapfuzz-fail-{seed0 + i}.txt"
            open(path, "wb").write(data)
            print(f"MISMATCH seed={seed0 + i} -> {path}")
            print(f"  status C={c[2]} R={r[2]}  stdout {len(c[0])} vs {len(r[0])}"
                  f"  stderr {c[1][:120]!r} vs {r[1][:120]!r}")
            if bad > 4:
                break
    print(f"{iterations - bad}/{iterations} identical")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
