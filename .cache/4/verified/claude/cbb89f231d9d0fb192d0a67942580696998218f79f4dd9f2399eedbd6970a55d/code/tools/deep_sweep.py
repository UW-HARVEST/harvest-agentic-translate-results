#!/usr/bin/env python3
"""Deep exhaustive sweeps over the alphabets glibc's %f collector branches on."""
import itertools
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
C_BIN = os.path.join(ROOT, "c_src", "build", "driver")
R_BIN = os.path.join(ROOT, "target", "release", "driver")


def run(binary, data: bytes) -> bytes:
    return subprocess.run([binary], input=data, stdout=subprocess.PIPE).stdout


def sweep(alpha: bytes, maxlen: int, prefixes):
    fails = 0
    total = 0
    for n in range(0, maxlen + 1):
        for combo in itertools.product(alpha, repeat=n):
            body = bytes(combo)
            for pre in prefixes:
                d = pre + body
                total += 1
                if run(C_BIN, d) != run(R_BIN, d):
                    fails += 1
                    print("MISMATCH %r" % d)
                    if fails > 20:
                        return total, fails
    return total, fails


SWEEPS = {
    # every character class of the numeric collector, depth 5
    "num5": (b"0x.p1e+-", 5, [b"", b"-"]),
    # hex digits + letters that are also hex digits, depth 4
    "hex4": (b"0xa.pfe1", 4, [b"", b"-", b"+"]),
    # the special-word letters mixed with digits, depth 4
    "word4": (b"infnaty1", 4, [b"", b"-"]),
}


def main():
    which = sys.argv[1] if len(sys.argv) > 1 else "num5"
    alpha, ln, pre = SWEEPS[which]
    t, f = sweep(alpha, ln, pre)
    print("sweep %s over %r len<=%d: %d cases, %d mismatches" % (which, alpha, ln, t, f))
    return 1 if f else 0


if __name__ == "__main__":
    sys.exit(main())
