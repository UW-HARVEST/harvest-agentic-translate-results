#!/usr/bin/env python3
"""Differential test: C reference vs Rust translation (incremental logging)."""
import random
import subprocess
import sys
import os

BASE = os.path.dirname(os.path.abspath(__file__))
CDIR = os.path.join(BASE, "c")
RDIR = os.path.join(BASE, "rs")
LOG = open(os.path.join(BASE, "dt3.log"), "w", buffering=1)

fails = 0
total = 0


def run(d, args, stdin_bytes):
    try:
        p = subprocess.run(["./driver"] + args, cwd=d, input=stdin_bytes,
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                           timeout=10)
        return p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        return "TIMEOUT", b"", b""


def T(args, stdin_bytes=b"", label=""):
    global fails, total
    total += 1
    a = run(CDIR, args, stdin_bytes)
    b = run(RDIR, args, stdin_bytes)
    if a != b:
        fails += 1
        LOG.write("MISMATCH %s args=%r stdin=%r\n" % (label, args, stdin_bytes))
        LOG.write("  C : %r\n" % (a,))
        LOG.write("  RS: %r\n" % (b,))
    if total % 500 == 0:
        LOG.write("progress total=%d fails=%d\n" % (total, fails))


# ---- hand written cases -------------------------------------------------
T([])
T(["--help"])
T(["--help", "1", "2"])
T(["1", "--help"])
T(["--stdin"])
T(["--stdin"], b"0 5 0 7 1 5 10\n")
T(["--stdin"], b"0 5 0 7 1 5 10")
T(["--stdin"], b"  0\t5\r\n 1 \n\n2\n")
T(["--stdin"], b"1 2 abc 3 0x10 -4 +5 \n")
T(["--stdin"], b"\n\n\n")
T(["--stdin"], b"12abc 7\n")
T(["--stdin"], b"9999999999999999999999 5\n")
T(["--stdin"], b"-9999999999999999999999 5\n")
T(["--stdin"], b"4294967296 4294967297\n")
T(["--stdin"], b"0 1\x002 3\n4 5\n")
T(["--stdin"], b"\x00 1 2\n")
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
T(["9223372036854775807"])
T(["9223372036854775808"])
T(["-9223372036854775808"])
T(["\t42"])
T(["\v42"])
T(["\f42"])
T(["\n42"])
T(["42\n"])
T(["4294967296"])
T(["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11"])

# every opcode alone and with small operands
for op in range(-3, 14):
    T([str(op)])
    for x in (-2, -1, 0, 1, 2, 3, 5, 7, 10, 42, 100):
        T([str(op), str(x)])
        T([str(op), str(x), "10"])
        T(["0", str(x), str(op)])
        T(["0", str(x), "3", str(op)])
        T(["0", str(x), "0", "9", str(op)])
        T(["0", str(x), "0", "3", "9", str(op), "1"])

# op 9 (stream) with varying m and stack depths
for m in range(-1, 7):
    for depth in range(0, 7):
        prog = []
        for d in range(depth):
            prog += ["0", str(d * 7 + 3)]
        prog += ["9", str(m)]
        T(prog, label="op9 m=%d depth=%d" % (m, depth))

# op 7 (repeat) with small counts over every inner opcode
for times in range(-2, 6):
    for inner in range(-1, 12):
        T(["0", "5", "0", "9", "7", str(times), str(inner), "1"],
          label="op7 t=%d i=%d" % (times, inner))

# op 6 (jump) boundaries
for k in range(-2, 8):
    T(["0", "1", "6", str(k), "3", "3", "3", "10"], label="op6 k=%d" % k)
    T(["0", "0", "6", str(k), "3", "3", "3", "10"], label="op6z k=%d" % k)

# long fgets line splitting (> 4095 bytes)
big = b" ".join(b"%d" % (i % 11) for i in range(5000))
T(["--stdin"], big + b"\n")
nums = []
n = 0
while sum(len(x) + 1 for x in nums) < 4200:
    nums.append(b"%d" % (12345678 + n))
    n += 1
T(["--stdin"], b" ".join(nums) + b"\n")
T(["--stdin"], b"1" * 4100 + b"\n")
T(["--stdin"], b"0 " * 2100 + b"\n")
pad = b"0 " * 2045
T(["--stdin"], pad + b"123456789 7\n")
for k in range(4080, 4102):
    T(["--stdin"], b"0" * k + b" 3 3\n", label="straddleA%d" % k)
    T(["--stdin"], b"3 " * (k // 2) + b"1234567890123\n", label="straddleB%d" % k)
T(["--stdin"], b"0 3\n" * 1500)

# ---- random fuzzing, small values (op 7 counts stay tiny) ---------------
random.seed(98765)
SMALL_OPS = list(range(-3, 16))
for it in range(5000):
    ln = random.randint(1, 40)
    args = []
    for _ in range(ln):
        r = random.random()
        if r < 0.7:
            args.append(str(random.choice(SMALL_OPS)))
        else:
            args.append(str(random.randint(-64, 64)))
    T(args, label="fuzzA%d" % it)

# ---- random fuzzing, wide values but never the literal opcode 7 --------
WIDE = [-2147483648, 2147483647, 1000000, -1000000, 65536, 255, 1073741824,
        -1073741824, 12345, -99999]
OPS_NO7 = [0, 1, 2, 3, 4, 5, 6, 8, 9, 10, 11, -1, 12, 99]
for it in range(2000):
    ln = random.randint(1, 14)
    args = []
    for _ in range(ln):
        r = random.random()
        if r < 0.55:
            args.append(str(random.choice(OPS_NO7)))
        elif r < 0.8:
            args.append(str(random.choice(WIDE)))
        else:
            v = random.randint(-300, 300)
            while v == 7:
                v = random.randint(-300, 300)
            args.append(str(v))
    T(args, label="fuzzB%d" % it)

# ---- random tokens through stdin ---------------------------------------
for it in range(1500):
    toks = []
    for _ in range(random.randint(0, 12)):
        r = random.random()
        if r < 0.6:
            toks.append(b"%d" % random.choice(SMALL_OPS))
        elif r < 0.7:
            toks.append(b"%d" % random.randint(-50, 50))
        elif r < 0.82:
            toks.append(random.choice([b"abc", b"", b"0x5", b"5x", b"--", b"+",
                                       b"-", b" ", b"\v7", b"7\v", b".5", b"1e3"]))
        else:
            toks.append(b"%d" % random.choice(WIDE))
    sep = [b" ", b"\t", b"\n", b"\r\n", b"  ", b"\r"]
    data = b""
    for t in toks:
        data += t + random.choice(sep)
    T(["--stdin"], data, label="rfuzz%d" % it)

LOG.write("DONE total=%d fails=%d\n" % (total, fails))
LOG.close()
sys.exit(1 if fails else 0)
