#!/usr/bin/env python3
"""Randomised differential fuzzing of the C reference vs. the Rust port."""
import os
import random
import subprocess
import sys

W = "$HARVEST_WORKDIR"
C = W + "/_ref/driver"
R = W + "/translation/target/release/driver"
DATA = W + "/_ref/data"

files = [os.path.join(DATA, f) for f in sorted(os.listdir(DATA))]
files += ["/nonexistent", "", "   ", DATA, "/etc/hostname"]

WORDS = [
    "int", "float", "if", "else", "while", "return", "x", "y", "foo", "bar_1",
    "0", "42", "3.14", "1.2.3", "+", "-", "*", "/", "%", "==", "!=", "<=", ">=",
    "&&", "||", "++", "--", "->", "<<", ">>", "=", "<", ">", "!", "&", "|", "^",
    "~", "?", ":", "(", ")", "{", "}", "[", "]", ";", ",", ".", "@", "#", "$",
    '"str"', "'c'", '"esc\\"q"', "'", '"', "//comment", "/*ml*/", "/*open",
    "_ident", "a" * 300, "\\", "\t", "\x0b", "\x0c", "\r", "\xff", "\xe9",
    "*/", "9999999999999999999999", ".5", "0.", "sizeof", "typedef",
]


def rand_text(rng):
    lines = []
    for _ in range(rng.randint(0, 6)):
        n = rng.randint(0, 12)
        line = " ".join(rng.choice(WORDS) for _ in range(n))
        if rng.random() < 0.1:
            line = line * rng.randint(1, 40)
        lines.append(line)
    return "\n".join(lines) + "\n\n"


def rand_input(rng):
    parts = []
    for _ in range(rng.randint(1, 8)):
        c = rng.choice([1, 1, 2, 3, 4, 5, 6, 6, 0, 8, 99, -3])
        if rng.random() < 0.08:
            parts.append(rng.choice(["abc\n", "\n", "  \n", "1.7\n", "3q\n", "+2\n",
                                     "99999999999999999999\n", "\t5\n"]))
            continue
        parts.append("%d\n" % c)
        if c == 1 or c == 6:
            parts.append(rand_text(rng))
        elif c == 2:
            parts.append(rng.choice(files) + "\n")
        elif c == 5:
            parts.append(rng.choice(["", "x", "int", "\"", "/", "  ", "a" * 300,
                                     "\xff", "="]) + "\n")
    if rng.random() < 0.7:
        parts.append("7\n")
    return "".join(parts).encode("latin-1")


def run(binary, data):
    p = subprocess.run([binary], input=data, stdout=subprocess.PIPE,
                       stderr=subprocess.PIPE)
    return p.returncode, p.stdout, p.stderr


def main():
    iterations = int(sys.argv[1]) if len(sys.argv) > 1 else 300
    seed = int(sys.argv[2]) if len(sys.argv) > 2 else 1234
    rng = random.Random(seed)
    fails = 0
    for i in range(iterations):
        data = rand_input(rng)
        rc, ro, re = run(C, data)
        rc2, ro2, re2 = run(R, data)
        if (rc, ro, re) != (rc2, ro2, re2):
            fails += 1
            name = W + "/_ref/fuzzfail_%d" % i
            with open(name, "wb") as f:
                f.write(data)
            print("FAIL iter %d -> %s" % (i, name))
            if ro != ro2:
                print("  stdout differs (%d vs %d bytes)" % (len(ro), len(ro2)))
                for a, b in zip(ro.split(b"\n"), ro2.split(b"\n")):
                    if a != b:
                        print("   C: %r" % a[:120])
                        print("   R: %r" % b[:120])
                        break
            if re != re2:
                print("  stderr differs: %r vs %r" % (re[:200], re2[:200]))
            if rc != rc2:
                print("  exit differs: %d vs %d" % (rc, rc2))
            if fails > 5:
                break
    print("iterations=%d fails=%d" % (iterations, fails))
    return 1 if fails else 0


sys.exit(main())
