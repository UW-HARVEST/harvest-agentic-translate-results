#!/usr/bin/env python3
"""Exhaustive short-string differential sweep for the C and Rust drivers.

Enumerates every string of length <= MAXLEN over a small alphabet that covers
every branch of glibc's %f collection state machine, optionally prefixed with a
sign, and compares the two programs' stdout byte-for-byte.
"""
import itertools
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
C_BIN = os.path.join(ROOT, "c_src", "build", "driver")
R_BIN = os.path.join(ROOT, "target", "release", "driver")


def run(binary, data: bytes) -> bytes:
    p = subprocess.run([binary], input=data, stdout=subprocess.PIPE)
    return p.stdout


def sweep(alphabet, maxlen, prefixes):
    fails = 0
    total = 0
    for n in range(0, maxlen + 1):
        for combo in itertools.product(alphabet, repeat=n):
            body = bytes(combo)
            for pre in prefixes:
                data = pre + body
                total += 1
                c = run(C_BIN, data)
                r = run(R_BIN, data)
                if c != r:
                    fails += 1
                    print("MISMATCH %r: C=%r Rust=%r" % (data, c, r))
                    if fails > 30:
                        print("too many failures")
                        return total, fails
    return total, fails


def main():
    which = sys.argv[1] if len(sys.argv) > 1 else "core"
    if which == "core":
        # every character class the collection loop distinguishes
        alpha = b"0x.pe1-+"
        total, fails = sweep(alpha, 4, [b"", b"-"])
    elif which == "wide":
        alpha = b"0123456789abcdefxXpPeE.+-_,int"
        total, fails = sweep(alpha, 2, [b"", b"-", b"+"])
    elif which == "words":
        alpha = b"infntyaINFTYX0.1"
        total, fails = sweep(alpha, 3, [b"", b"-", b"i", b"in", b"inf", b"na", b"nan"])
    else:
        print("unknown sweep")
        return 2
    print("sweep=%s checked %d cases, %d mismatches" % (which, total, fails))
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
