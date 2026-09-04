#!/usr/bin/env python3
"""Differential test: C reference vs Rust translation."""
import random
import subprocess
import sys
import os

BASE = os.path.dirname(os.path.abspath(__file__))
CDIR = os.path.join(BASE, "c")
RDIR = os.path.join(BASE, "rs")


def run(d, args, stdin_bytes):
    p = subprocess.run(["./driver"] + args, cwd=d, input=stdin_bytes,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return p.returncode, p.stdout, p.stderr


def check(args, stdin_bytes, label=""):
    a = run(CDIR, args, stdin_bytes)
    b = run(RDIR, args, stdin_bytes)
    if a != b:
        print("MISMATCH %s args=%r stdin=%r" % (label, args, stdin_bytes))
        print("  C : rc=%d out=%r err=%r" % a)
        print("  RS: rc=%d out=%r err=%r" % b)
        return False
    return True


fails = 0
total = 0


def T(args, stdin_bytes=b"", label=""):
    global fails, total
    total += 1
    if not check(args, stdin_bytes, label):
        fails += 1


# ---- hand written cases -------------------------------------------------
T([])
T(["--help"])
T(["--help", "1", "2"])
T(["1", "--help"])
T(["--stdin"])
T(["--stdin"], b"0 5 0 7 1 5 10\n")
T(["--stdin"], b"0 5 0 7 1 5 10")           # no trailing newline
T(["--stdin"], b"  0\t5\r\n 1 \n\n2\n")
T(["--stdin"], b"1 2 abc 3 0x10 -4 +5 \n")
T(["--stdin"], b"\n\n\n")
T(["--stdin"], b"12abc 7\n")
T(["--stdin"], b"9999999999999999999999 5\n")
T(["--stdin"], b"-9999999999999999999999 5\n")
T(["--stdin"], b"4294967296 4294967297\n")
T(["--stdin"], b"0 1\x002 3\n4 5\n")        # embedded NUL
T(["--stdin", "3"], b"5 8\n")
T(["3", "--stdin", "5"], b"8 1 2\n")
T(["abc"])
T([""])
T(["", "5"])
T([" 12"])
T(["12 "])
T(["+7"])
T(["-7"])
T(["--"])
T(["--std"])
T(["0x10"])
T(["99999999999999999999"])
T(["-99999999999999999999"])
T(["2147483648"])
T(["-2147483649"])
T(["\t42"])
T(["\v42"])
T(["\f42"])
T(["\n42"])
T(["42\n"])

# every single opcode alone and with operands
for op in range(-3, 14):
    T([str(op)])
    for x in (-2, -1, 0, 1, 2, 3, 5, 7, 10, 42, 100):
        T([str(op), str(x)])
        T([str(op), str(x), "10"])
        T(["0", str(x), str(op)])
        T(["0", str(x), "3", str(op)])
        T(["0", str(x), "0", "9", str(op)])

# long fgets line splitting (> 4095 bytes, splitting a number)
big = b" ".join(b"%d" % (i % 11) for i in range(3000))
T(["--stdin"], big + b"\n")
nums = []
n = 0
while sum(len(x) + 1 for x in nums) < 4200:
    nums.append(b"%d" % (12345678 + n))
    n += 1
T(["--stdin"], b" ".join(nums) + b"\n")
T(["--stdin"], b"1" * 4100 + b"\n")
T(["--stdin"], b"0 " * 2100 + b"\n")
# a number straddling the 4095 byte boundary
pad = b"0 " * 2045          # 4090 bytes
T(["--stdin"], pad + b"123456789 7\n")
for k in range(4085, 4100):
    T(["--stdin"], b"0 " * (k // 2) + b"7" * 12 + b"\n", label="straddle%d" % k)

# ---- random fuzzing ----------------------------------------------------
random.seed(1234)
OPS = list(range(0, 12)) + [-1, 12, 99]
for it in range(4000):
    ln = random.randint(1, 14)
    args = []
    for _ in range(ln):
        r = random.random()
        if r < 0.62:
            args.append(str(random.choice(OPS)))
        elif r < 0.8:
            args.append(str(random.randint(-6, 6)))
        elif r < 0.92:
            args.append(str(random.randint(-100, 100)))
        else:
            args.append(str(random.choice([-2147483648, 2147483647, 1000000,
                                           -1000000, 65536, 255])))
    T(args, b"", label="fuzz%d" % it)

# random with weird tokens through stdin
for it in range(1200):
    toks = []
    for _ in range(random.randint(0, 12)):
        r = random.random()
        if r < 0.6:
            toks.append(b"%d" % random.choice(OPS))
        elif r < 0.7:
            toks.append(b"%d" % random.randint(-50, 50))
        elif r < 0.8:
            toks.append(random.choice([b"abc", b"", b"0x5", b"5x", b"--", b"+", b"-"]))
        else:
            toks.append(b"%d" % random.randint(-2 ** 40, 2 ** 40))
    sep = [b" ", b"\t", b"\n", b"\r\n", b"  ", b"\r"]
    data = b""
    for t in toks:
        data += t + random.choice(sep)
    T(["--stdin"], data, label="rfuzz%d" % it)

print("total=%d fails=%d" % (total, fails))
sys.exit(1 if fails else 0)
