#!/usr/bin/env python3
"""Quick differential fuzzer: C driver vs Rust driver (executables).

Usage: python3 fuzz_diff.py [iterations] [seed]
"""
import random
import subprocess
import sys
import os

C = os.path.abspath("c_src/build/driver")
R = os.path.abspath("target/release/driver")

UPPER = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
ALNUM = UPPER + "0123456789"


def run(binary, args, data):
    p = subprocess.run([binary] + args, input=data, stdout=subprocess.PIPE,
                       stderr=subprocess.PIPE, timeout=20)
    return p.returncode, p.stdout, p.stderr


def rand_ts(rng):
    kind = rng.randrange(10)
    if kind == 0:
        return str(rng.randrange(0, 3))
    if kind == 1:
        return str(rng.randrange(0, 2**32))
    if kind == 2:
        return "-" + str(rng.randrange(0, 2**33))
    if kind == 3:
        return "+" + str(rng.randrange(0, 2**32))
    if kind == 4:
        return str(rng.randrange(2**62, 2**70))
    if kind == 5:
        return "0" * rng.randrange(1, 5) + str(rng.randrange(0, 100))
    if kind == 6:
        return str(rng.choice([0, 1, 2147483647, 2147483648, 4294967295,
                               4294967296, 9223372036854775807,
                               9223372036854775808]))
    if kind == 7:
        return "-" + str(rng.choice([1, 2147483648, 9223372036854775808,
                                     9223372036854775809]))
    if kind == 8:
        return "".join(rng.choice("0123456789") for _ in range(rng.randrange(1, 30)))
    return str(rng.randrange(0, 1000))


def rand_field(rng, alphabet, maxlen, allow_bad=True):
    n = rng.randrange(0, maxlen + 3)
    s = "".join(rng.choice(alphabet) for _ in range(n))
    if allow_bad and rng.randrange(12) == 0:
        # inject an out-of-set char
        pos = rng.randrange(0, len(s) + 1)
        bad = rng.choice("abcxyz_-.!/*#%")
        s = s[:pos] + bad + s[pos:]
    return s


def rand_comment(rng):
    kind = rng.randrange(6)
    if kind == 0:
        return ""
    if kind == 1:
        return " " * rng.randrange(1, 4)
    n = rng.randrange(0, 90)
    pool = ALNUM + "abcdef   \t,.;:/-_()[]#*%\r\x0b\x0c\x01\xff"
    return "".join(rng.choice(pool) for _ in range(n))


def gen_record(rng):
    ts = rand_ts(rng)
    lug = rand_field(rng, ALNUM, 8)
    fl = rand_field(rng, ALNUM, 6)
    dep = rand_field(rng, UPPER, 3)
    arr = rand_field(rng, UPPER, 3)
    com = rand_comment(rng)
    sep = lambda: rng.choice([" ", "  ", " ", "\t", " ", "\n", " "])
    return ts + sep() + lug + sep() + fl + sep() + dep + sep() + arr + com


def gen_structured(rng):
    """Well-formed-ish input built from a small pool so supersedes/matches fire."""
    lugs = ["".join(rng.choice(ALNUM) for _ in range(rng.randrange(1, 9)))
            for _ in range(rng.randrange(1, 4))]
    fls = ["".join(rng.choice(ALNUM) for _ in range(rng.randrange(1, 7)))
           for _ in range(rng.randrange(1, 4))]
    aps = ["".join(rng.choice(UPPER) for _ in range(rng.randrange(1, 4)))
           for _ in range(rng.randrange(1, 4))]
    lines = []
    for _ in range(rng.randrange(0, 12)):
        ts = rng.choice([str(rng.randrange(0, 5)), str(rng.randrange(0, 10**9))])
        lines.append("%s %s %s %s %s%s" % (
            ts, rng.choice(lugs), rng.choice(fls), rng.choice(aps),
            rng.choice(aps), rand_comment(rng)))
    data = "\n".join(lines)
    if rng.randrange(4):
        data += "\n"
    return data


def gen_input(rng):
    kind = rng.randrange(10)
    if kind < 4:
        return gen_structured(rng).encode("latin1")
    if kind < 8:
        recs = [gen_record(rng) for _ in range(rng.randrange(0, 8))]
        data = "\n".join(recs)
        if rng.randrange(3):
            data += "\n"
        return data.encode("latin1")
    if kind == 8:
        return bytes(rng.randrange(0, 256) for _ in range(rng.randrange(0, 60)))
    pool = b"0123456789ABCXYZ abc\n\t-+[]"
    return bytes(rng.choice(pool) for _ in range(rng.randrange(0, 80)))


def gen_args(rng, data):
    words = []
    try:
        text = data.decode("latin1")
        for tok in text.replace("\n", " ").split(" "):
            tok = tok.replace("\x00", "")
            if tok:
                words.append(tok[:8])
    except Exception:
        pass
    def one():
        k = rng.randrange(10)
        if k < 5:
            return "-"
        if k < 7 and words:
            return rng.choice(words)
        if k == 7:
            return ""
        if k == 8:
            return "-" + "".join(rng.choice(ALNUM) for _ in range(rng.randrange(0, 4)))
        return "".join(rng.choice(ALNUM) for _ in range(rng.randrange(0, 5)))
    return [one() for _ in range(4)]


def main():
    iters = int(sys.argv[1]) if len(sys.argv) > 1 else 500
    seed = int(sys.argv[2]) if len(sys.argv) > 2 else 12345
    rng = random.Random(seed)
    fails = 0
    for i in range(iters):
        data = gen_input(rng)
        args = gen_args(rng, data)
        try:
            c = run(C, args, data)
        except subprocess.TimeoutExpired:
            print("C TIMEOUT", args, data)
            continue
        try:
            r = run(R, args, data)
        except subprocess.TimeoutExpired:
            print("RUST TIMEOUT", args, data)
            fails += 1
            continue
        if c != r:
            fails += 1
            print("=== MISMATCH #%d (iter %d) ===" % (fails, i))
            print("args:", args)
            print("stdin:", repr(data))
            print("C   rc=%d out=%r err=%r" % c)
            print("RS  rc=%d out=%r err=%r" % r)
            if fails >= 15:
                print("too many mismatches, stopping")
                break
    print("done: %d iterations, %d mismatches" % (i + 1, fails))
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
