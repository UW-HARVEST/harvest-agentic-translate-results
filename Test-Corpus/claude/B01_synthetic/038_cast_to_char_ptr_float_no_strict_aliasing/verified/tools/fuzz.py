#!/usr/bin/env python3
"""Quick differential fuzzer for the C and Rust `driver` executables.

Usage: fuzz.py [n_random] [seed]
"""
import os
import random
import struct
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
C_BIN = os.path.join(ROOT, "c_src", "build", "driver")
R_BIN = os.path.join(ROOT, "target", "release", "driver")


def run(binary, data: bytes) -> bytes:
    p = subprocess.run([binary], input=data, stdout=subprocess.PIPE,
                       stderr=subprocess.PIPE)
    return p.stdout + b"|rc=" + str(p.returncode).encode()


EDGE = [
    b"", b" ", b"\n", b"\t\n\v\f\r ", b"\0", b"\0\01.5",
    b"abc", b"x", b"-", b"+", b".", b"-.", b"+.", b"e5", b"E", b"-e", b"()",
    b"_", b"0x", b"0X", b"-0x", b"0xp1", b"0x.p1", b"0x.", b"0x.p", b"0xg",
    b"nan", b"-nan", b"+nan", b"NAN", b"NaN", b"nan(", b"nan()", b"nan(123)",
    b"nan(0x1f)", b"-nan(1)", b"nanx", b"na", b"n",
    b"inf", b"-inf", b"+inf", b"INF", b"Inf", b"infinity", b"-INFINITY",
    b"Infinity", b"infin", b"in", b"i", b"infinit", b"infinityx",
    b"0", b"-0", b"+0", b"0.0", b"-0.0", b"0.", b".0", b"1", b"-1",
    b"1.", b"1.e5", b".e5", b".5", b"1e", b"1e+", b"1e-", b"1E5", b"1e5",
    b"1e39", b"-1e39", b"3.4028235e38", b"3.4028236e38", b"3.4028234e38",
    b"1e400", b"1e-400", b"1e-46", b"1e-45", b"7e-46", b"1.4e-45",
    b"1.1754944e-38", b"1.1754942e-38", b"5.877472e-39", b"1e-38",
    b"1e1000000", b"1e1000001", b"1e-1000001", b"1e99999999999999999999",
    b"1e-99999999999999999999",
    b"0x1p0", b"0x1p-149", b"0x1p-150", b"0x0.8p-148", b"0x1.fffffep127",
    b"0x1.ffffffp127", b"0x1p128", b"0x1p-1000", b"0x1p1000",
    b"0xabcdefp-20", b"0X1.8P+3", b"0x1p", b"0x1p+", b"0x1p-", b"0x1.p2",
    b"0x.8p1", b"0x10", b"0x1.8p1", b"-0x0", b"0x0", b"0x00000001p0",
    b"1.5 2.5", b"\n\n\n 42", b"1.5abc", b"1_000", b"1,5",
    b"12345678901234567890", b"000000000000001.5", b"--1", b"+-1", b"1-2",
    b"1+2", b"1.5.5", b"1..5", b"1e5e5", b"0x1p1p1",
    b"  \n\t  -3.25e-2  ", b"1e0", b"9" * 100, b"9" * 1000,
    b"0." + b"0" * 100 + b"1", b"1" + b"0" * 100,
    b"0x" + b"f" * 40 + b"p-160",
    b"16777215", b"16777216", b"16777217", b"16777218", b"16777219",
    b"8388608.5", b"8388609.5", b"33554431", b"33554433",
    b"1.00000005960464477539062", b"1.00000011920928955078125",
    b"2.3509886e-38", b"1.7014118e38", b"340282346638528859811704183484516925440",
    b"340282356779733661637539395458142568448",
    b"340282356779733661637539395458142568447",
]


def gen_random(rnd):
    kind = rnd.randrange(12)
    if kind == 0:
        b = rnd.getrandbits(32)
        f = struct.unpack("<f", struct.pack("<I", b))[0]
        return repr(f).encode()
    if kind == 1:
        b = rnd.getrandbits(32)
        f = struct.unpack("<f", struct.pack("<I", b))[0]
        return ("%.*g" % (rnd.randrange(1, 25), f)).encode()
    if kind == 2:
        b = rnd.getrandbits(32)
        f = struct.unpack("<f", struct.pack("<I", b))[0]
        return float.hex(f).encode()
    if kind == 3:  # random decimal digits + exponent
        n = rnd.randrange(1, 30)
        s = "".join(rnd.choice("0123456789") for _ in range(n))
        if rnd.random() < 0.7:
            m = rnd.randrange(0, 20)
            s += "." + "".join(rnd.choice("0123456789") for _ in range(m))
        if rnd.random() < 0.7:
            s += rnd.choice("eE") + rnd.choice(["", "+", "-"]) + str(rnd.randrange(0, 60))
        if rnd.random() < 0.3:
            s = rnd.choice("+-") + s
        return s.encode()
    if kind == 4:  # random hex float
        n = rnd.randrange(1, 12)
        s = "0" + rnd.choice("xX") + "".join(rnd.choice("0123456789abcdefABCDEF") for _ in range(n))
        if rnd.random() < 0.6:
            m = rnd.randrange(0, 10)
            s += "." + "".join(rnd.choice("0123456789abcdefABCDEF") for _ in range(m))
        if rnd.random() < 0.8:
            s += rnd.choice("pP") + rnd.choice(["", "+", "-"]) + str(rnd.randrange(0, 200))
        if rnd.random() < 0.3:
            s = rnd.choice("+-") + s
        return s.encode()
    if kind == 5:  # near-halfway decimal
        b = rnd.getrandbits(32) & 0x7FFFFFFF
        f = struct.unpack("<f", struct.pack("<I", b))[0]
        try:
            d = float(f)
        except Exception:
            return b"0"
        return ("%.40e" % d).encode()
    if kind == 6:  # garbage ascii
        n = rnd.randrange(0, 12)
        return bytes(rnd.choice(b"0123456789abcdefxXpPeE+-._() \t\n\rinfaN") for _ in range(n))
    if kind == 7:  # random bytes
        n = rnd.randrange(0, 16)
        return bytes(rnd.randrange(256) for _ in range(n))
    if kind == 8:  # whitespace prefix + number
        ws = bytes(rnd.choice(b" \t\n\v\f\r") for _ in range(rnd.randrange(0, 8)))
        return ws + gen_random(rnd)
    if kind == 9:  # long digit strings
        n = rnd.randrange(1, 400)
        s = "".join(rnd.choice("0123456789") for _ in range(n))
        s += "e" + rnd.choice(["", "-"]) + str(rnd.randrange(0, 400))
        return s.encode()
    if kind == 10:  # subnormal / boundary region
        e = rnd.randrange(-50, -30)
        m = rnd.randrange(1, 10**9)
        return ("%de%d" % (m, e)).encode()
    # exact halfway between two floats
    b = rnd.randrange(1, 0x7F7FFFFF)
    f0 = struct.unpack("<f", struct.pack("<I", b))[0]
    f1 = struct.unpack("<f", struct.pack("<I", b + 1))[0]
    mid = (float(f0) + float(f1)) / 2
    return ("%.60e" % mid).encode()


def main():
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 5000
    seed = int(sys.argv[2]) if len(sys.argv) > 2 else 12345
    rnd = random.Random(seed)
    cases = list(EDGE)
    for _ in range(n):
        cases.append(gen_random(rnd))
    fails = 0
    for data in cases:
        c = run(C_BIN, data)
        r = run(R_BIN, data)
        if c != r:
            fails += 1
            print("MISMATCH input=%r\n  C   =%r\n  Rust=%r" % (data, c, r))
            if fails > 40:
                print("... too many failures, stopping")
                break
    print("checked %d cases, %d mismatches" % (len(cases), fails))
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
