#!/usr/bin/env python3
"""Random differential fuzz: feed the same random command stream to both
programs and compare stdout, stderr and exit status. ASLR is disabled so that
reads of freed memory in the C program are reproducible."""
import random, subprocess, sys, os

ROOT = os.path.dirname(os.path.abspath(__file__))
C = [os.path.join(ROOT, "c_src/build/driver")]
R = [os.path.join(ROOT, "translation/target/release/driver")]
if subprocess.run(["setarch", "-R", "/bin/true"], capture_output=True).returncode == 0:
    C = ["setarch", "-R"] + C
    R = ["setarch", "-R"] + R

NAMES = [b"A", b"B", b"C", b"", b"8", b"x" * 63, b"x" * 64, b"x" * 70,
         b"  ", b"A\r", b"Zurich", b"N001", b"P0"]
DISTS = [b"0", b"1", b"5", b"-1", b"-0", b"2147483647", b"2147483648",
         b"4294967296", b"99999999999999999999", b"abc", b"", b"  7x", b"10"]
CHOICES = [b"1", b"2", b"3", b"4", b"5", b"6", b"7", b"0", b"9", b"-1",
           b"abc", b"", b"  3", b"4294967297", b"3abc"]

def gen(rng):
    out = []
    for _ in range(rng.randint(1, 60)):
        c = rng.choice(CHOICES)
        out.append(c)
        if c in (b"1", b"4", b"6", b"7"):
            out.append(rng.choice(NAMES))
        elif c == b"2":
            out += [rng.choice(NAMES), rng.choice(NAMES), rng.choice(DISTS)]
        elif c == b"5":
            out += [rng.choice(NAMES), rng.choice(NAMES)]
    if rng.random() < 0.5:
        out.append(b"8")
    data = b"\n".join(out)
    return data + (b"\n" if rng.random() < 0.9 else b"")

def run(cmd, data):
    p = subprocess.run(cmd, input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return p.stdout, p.stderr, p.returncode

def main():
    iterations = int(sys.argv[1]) if len(sys.argv) > 1 else 500
    seed0 = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    bad = 0
    for i in range(iterations):
        rng = random.Random(seed0 + i)
        data = gen(rng)
        co, ce, cs = run(C, data)
        ro, re, rs = run(R, data)
        if (co, ce, cs) != (ro, re, rs):
            bad += 1
            print(f"MISMATCH seed={seed0+i}")
            open(f"/tmp/fuzz-fail-{seed0+i}.txt", "wb").write(data)
            if co != ro:
                print("  stdout differs")
            if ce != re:
                print(f"  stderr: C={ce[:200]!r} R={re[:200]!r}")
            if cs != rs:
                print(f"  status: C={cs} R={rs}")
            if bad > 5:
                break
    print(f"{iterations - bad}/{iterations} identical")
    return 1 if bad else 0

if __name__ == "__main__":
    sys.exit(main())
