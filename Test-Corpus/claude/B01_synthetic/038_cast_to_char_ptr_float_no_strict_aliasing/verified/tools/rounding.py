#!/usr/bin/env python3
"""Rounding-focused differential sweep (decimal and hex float boundaries)."""
import os
import random
import struct
import subprocess
import sys
from fractions import Fraction

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
C_BIN = os.path.join(ROOT, "c_src", "build", "driver")
R_BIN = os.path.join(ROOT, "target", "release", "driver")


def run(binary, data: bytes) -> bytes:
    return subprocess.run([binary], input=data, stdout=subprocess.PIPE).stdout


def f32(bits):
    return struct.unpack("<f", struct.pack("<I", bits))[0]


def dec_exact(fr: Fraction, digits=80):
    """Exact-ish decimal string for a positive Fraction."""
    if fr == 0:
        return "0"
    # scale so we print `digits` significant digits without rounding surprises
    from decimal import Decimal, getcontext
    getcontext().prec = digits + 20
    return str(Decimal(fr.numerator) / Decimal(fr.denominator))


def cases_decimal_midpoints(rnd, n):
    out = []
    for _ in range(n):
        bits = rnd.randrange(0, 0x7F7FFFFF)
        a = Fraction(f32(bits))
        b = Fraction(f32(bits + 1))
        mid = (a + b) / 2
        for fr, tweak in ((mid, 0), (mid, -1), (mid, +1), (a, 0), (b, 0)):
            s = dec_exact(fr)
            if tweak:
                # nudge the last digit to land just above/below the midpoint
                s = s + ("1" if tweak > 0 else "")
                if tweak < 0:
                    # subtract a tiny amount by appending a smaller magnitude
                    s = dec_exact(fr - Fraction(1, 10 ** 60))
            out.append(s.encode())
            out.append(("-" + s).encode())
    return out


def cases_hex(rnd, n):
    out = []
    for _ in range(n):
        nd = rnd.randrange(1, 32)
        digs = "".join(rnd.choice("0123456789abcdef") for _ in range(nd))
        frac = "".join(rnd.choice("0123456789abcdef") for _ in range(rnd.randrange(0, 16)))
        e = rnd.randrange(-200, 200)
        s = "0x" + digs
        if frac:
            s += "." + frac
        s += "p%+d" % e
        out.append(s.encode())
        out.append(("-" + s).encode())
        out.append(s.upper().replace("0X", "0x").encode())
    return out


def cases_hex_boundaries():
    out = []
    # subnormal / normal / overflow boundaries expressed in hex
    for e in range(-160, -120):
        out.append(("0x1p%d" % e).encode())
        out.append(("0x1.8p%d" % e).encode())
        out.append(("0x1.fffffep%d" % e).encode())
        out.append(("0x1.ffffffp%d" % e).encode())
        out.append(("0x3p%d" % e).encode())
    for e in range(120, 140):
        out.append(("0x1p%d" % e).encode())
        out.append(("0x1.fffffep%d" % e).encode())
        out.append(("0x1.ffffffp%d" % e).encode())
        out.append(("0x1.ffffffep%d" % e).encode())
    # exact halves at the subnormal boundary
    for k in range(1, 40):
        out.append(("0x%xp-149" % k).encode())
        out.append(("0x%x.8p-149" % k).encode())
        out.append(("0x%x.80000001p-149" % k).encode())
        out.append(("0x%x.7fffffffp-149" % k).encode())
    return out


def cases_decimal_boundaries():
    out = []
    for e in range(-50, -30):
        for m in (1, 2, 3, 5, 7, 9, 14, 15, 17, 1401298464324817):
            out.append(("%de%d" % (m, e)).encode())
    for e in range(35, 42):
        for m in (1, 3, 34, 340, 3402823, 34028235, 34028236):
            out.append(("%de%d" % (m, e)).encode())
    out.append(b"1.40129846432481707e-45")
    out.append(b"7.00649232162408535e-46")
    out.append(b"7.00649232162408534e-46")
    out.append(b"7.00649232162408536e-46")
    out.append(b"3.40282356779733661637539395458142568448e38")
    out.append(b"3.40282356779733661637539395458142568447e38")
    out.append(b"1.17549421069244107548702944485e-38")
    return out


def main():
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 300
    seed = int(sys.argv[2]) if len(sys.argv) > 2 else 4242
    rnd = random.Random(seed)
    cases = []
    cases += cases_decimal_boundaries()
    cases += cases_hex_boundaries()
    cases += cases_hex(rnd, n)
    cases += cases_decimal_midpoints(rnd, max(1, n // 5))
    fails = 0
    for data in cases:
        c = run(C_BIN, data)
        r = run(R_BIN, data)
        if c != r:
            fails += 1
            print("MISMATCH %r: C=%r Rust=%r" % (data[:90], c, r))
            if fails > 30:
                print("too many")
                break
    print("checked %d cases, %d mismatches" % (len(cases), fails))
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
