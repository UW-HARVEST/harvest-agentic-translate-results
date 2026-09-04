#!/usr/bin/env python3
import os, random, subprocess, sys, itertools

D = os.path.dirname(os.path.abspath(__file__))


TIMEOUTS = []


def run(d, args, stdin=b""):
    try:
        p = subprocess.run(["./driver"] + args, cwd=os.path.join(D, d),
                           input=stdin, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, timeout=5)
    except subprocess.TimeoutExpired:
        return "TIMEOUT"
    return p.returncode, p.stdout, p.stderr


def cmp(args, stdin=b"", label=""):
    a = run("c", args, stdin)
    b = run("rs", args, stdin)
    if a == "TIMEOUT" and b == "TIMEOUT":
        TIMEOUTS.append(args)
        return True
    if a != b:
        print("MISMATCH", label, args, stdin[:120])
        print("  C :", a if a == "TIMEOUT" else "rc=%s out=%r err=%r" % a)
        print("  RS:", b if b == "TIMEOUT" else "rc=%s out=%r err=%r" % b)
        sys.stdout.flush()
        return False
    return True


cases = []
cases.append(([], b""))
cases.append((["--help"], b""))
cases.append((["--help", "1", "2"], b""))
cases.append((["--stdin"], b""))
cases.append((["--stdin"], b"0 5\n"))
cases.append((["abc"], b""))
cases.append((["12x", "--stdin"], b"1 2 3"))
cases.append((["10"], b""))
cases.append((["0"], b""))
cases.append(([""], b""))
cases.append((["99999999999999999999"], b""))
cases.append((["-99999999999999999999"], b""))
cases.append((["2147483648"], b""))
cases.append((["  42  "], b""))
cases.append((["\v42"], b""))
cases.append((["--stdin"], b"\v42\n"))
cases.append((["--stdin"], b"1\x002 3\n4 5\n"))
cases.append((["--stdin"], ("7 " * 3000).encode()))
cases.append((["--stdin"], b"0x10 010 +3 -0 5.5\n"))
for op in range(-2, 13):
    cases.append(([str(op)], b""))
    cases.append((["0", "5", str(op)], b""))

alpha = [-1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 99]
for n in (1, 2, 3):
    for t in itertools.product(alpha, repeat=n):
        cases.append(([str(x) for x in t], b""))

random.seed(1234)
for _ in range(4000):
    n = random.randint(1, 12)
    prog = [random.choice([random.randint(-3, 12), random.randint(-2 ** 31, 2 ** 31 - 1),
                           random.randint(-20, 20)]) for _ in range(n)]
    cases.append(([str(x) for x in prog], b""))

fails = 0
for i, (args, stdin) in enumerate(cases):
    if not cmp(args, stdin, "case%d" % i):
        fails += 1
        if fails > 15:
            break
print("ran", len(cases), "cases; failures:", fails, "; both-timeout:", len(TIMEOUTS))
for t in TIMEOUTS[:20]:
    print("  timeout:", t)
sys.exit(1 if fails else 0)
