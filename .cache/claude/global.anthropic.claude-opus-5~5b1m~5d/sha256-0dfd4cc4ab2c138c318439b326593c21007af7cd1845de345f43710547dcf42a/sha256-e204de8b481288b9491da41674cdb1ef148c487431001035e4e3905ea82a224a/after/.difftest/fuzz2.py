#!/usr/bin/env python3
"""Phase C: targeted sweeps over paths the first fuzz round under-covered."""
import os, random, subprocess, sys, itertools

D = os.path.dirname(os.path.abspath(__file__))


def run(d, args, stdin=b""):
    try:
        p = subprocess.run([b"./driver"] + args, cwd=os.path.join(D, d),
                           input=stdin, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, timeout=10)
    except subprocess.TimeoutExpired:
        return "TIMEOUT"
    return p.returncode, p.stdout, p.stderr


def cmp(args, stdin=b"", label=""):
    args = [a if isinstance(a, bytes) else str(a).encode() for a in args]
    a = run(os.environ.get("CDIR","c"), args, stdin)
    b = run("rs", args, stdin)
    if a == "TIMEOUT" and b == "TIMEOUT":
        return True
    if a != b:
        print("MISMATCH", label, args, stdin[:120])
        print("  C :", a if a == "TIMEOUT" else "rc=%s out=%r err=%r" % a)
        print("  RS:", b if b == "TIMEOUT" else "rc=%s out=%r err=%r" % b)
        sys.stdout.flush()
        return False
    return True


cases = []

# 1. nested LOOP body = every opcode, with and without a preloaded stack
for inner in range(-2, 13):
    for times in (0, 1, 2, 3, 5):
        cases.append(([7, times, inner], b""))
        cases.append(([0, 4, 0, 9, 7, times, inner], b""))
        cases.append(([0, 4, 0, 9, 7, times, inner, 1, 9, 2], b""))

# 2. STREAM with deep stacks: m from 0..6 with stack depth 0..8
for depth in range(0, 9):
    prog = []
    for i in range(depth):
        prog += [0, i * 3 - 4]
    for m in range(0, 7):
        cases.append((prog + [9, m], b""))
        cases.append((prog + [9, m, 9, 1], b""))

# 3. conditional jump k across the whole valid/invalid boundary
for k in range(-3, 8):
    for cond in (0, 1, -1, 2):
        cases.append(([0, cond, 6, k, 3, 3, 4, 10], b""))
        cases.append(([0, cond, 6, k], b""))

# 4. classify over a wide value range for both op 5 and op 8, repeated
for x in list(range(-40, 60)) + [2 ** 31 - 1, -2 ** 31, 1 << 20, -(1 << 20), 255, 256]:
    cases.append(([0, x, 5], b""))
    cases.append(([0, x, 8], b""))
    cases.append(([0, x, 5, 8, 5, 8, 5], b""))
    cases.append(([0, x, 3, 9, 2], b""))
    cases.append(([0, x, 3, 3, 9, 3], b""))

# 5. non-UTF8 / odd argv bytes
cases.append(([b"\xff\xfe"], b""))
cases.append(([b"\xff\xfe", b"5"], b""))
cases.append(([b"1\xff"], b""))
cases.append(([b"--stdin\xff"], b""))
cases.append(([b"-\xc3"], b""))
cases.append(([b"5", b"\x80\x80"], b""))

# 6. non-UTF8 / odd stdin bytes
cases.append(([b"--stdin"], b"\xff\xfe 5 3\n"))
cases.append(([b"--stdin"], b"3\x0b4 5\n"))
cases.append(([b"--stdin"], b"\r\r\r5\r\r\n"))
cases.append(([b"--stdin"], b"5\r\n\r\n3\n"))
cases.append(([b"--stdin"], b"  \n\n\n 0 5 \n"))
cases.append(([b"--stdin"], b"9223372036854775807 -9223372036854775808\n"))
cases.append(([b"--stdin"], b"5" * 40 + b"\n"))
cases.append(([b"--stdin"], b"\x00"))
cases.append(([b"--stdin"], b"0 5\x00 3\n0 6\n"))

# fgets chunk boundary sweep: token straddling the 4095-byte cut
for pad in range(4088, 4100):
    cases.append(([b"--stdin"], b" " * pad + b"12345\n0 5\n"))

# 7. many arguments (exercises the IntVec growth path repeatedly)
cases.append(([3] * 500, b""))
cases.append(([0, 5] * 300, b""))
cases.append(([5] * 200, b""))
cases.append(([8] * 200, b""))
cases.append(([0, 7] + [5, 4] * 100, b""))

# 8. stdin ignored unless --stdin is given
cases.append(([3], b"9 9 9\n"))
cases.append(([], b"9 9 9\n"))
cases.append(([b"--help"], b"9 9 9\n"))

# 9. random deep programs biased to opcodes, small loop counts
random.seed(99)
for _ in range(2500):
    n = random.randint(1, 24)
    prog = []
    for _ in range(n):
        if prog and prog[-1] == 7:
            v = random.randint(-2, 5)
        else:
            r = random.random()
            if r < 0.7:
                v = random.randint(-1, 11)
            elif r < 0.9:
                v = random.randint(-30, 30)
            else:
                v = random.randint(-2 ** 31, 2 ** 31 - 1)
        prog.append(v)
    cases.append((prog, b""))

fails = 0
for i, (args, stdin) in enumerate(cases):
    if not cmp(args, stdin, "case%d" % i):
        fails += 1
        if fails > 10:
            break
print("ran", len(cases), "cases; failures:", fails)
sys.exit(1 if fails else 0)
