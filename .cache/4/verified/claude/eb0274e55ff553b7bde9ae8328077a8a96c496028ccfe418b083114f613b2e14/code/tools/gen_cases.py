#!/usr/bin/env python3
"""Adversarial + randomized case generation for the %f differential test."""
import random
import struct

FIXED = [
    # --- empty / whitespace / EOF ---
    "", " ", "   ", "\n", "\t", "\r\n", "\x0b", "\x0c", "  \n\t \v\f\r  ",
    # --- plain integers ---
    "0", "1", "9", "10", "42", "-0", "-1", "+1", "+0", "007", "0000",
    "16777216", "16777217", "16777218", "33554431", "33554433",
    "2147483647", "4294967295", "18446744073709551616",
    # --- decimals ---
    "0.0", "-0.0", "+0.0", "1.5", "-1.5", ".5", "-.5", "+.5", "5.", "-5.",
    "0.1", "0.2", "0.3", "3.14159265358979", "2.718281828459045",
    "1.0000000000000000000000001", "0.000000000000000000000001",
    # --- exponents ---
    "1e0", "1e1", "1e-1", "1E5", "1e+5", "1e-5", "1e10", "1e38", "1e39",
    "1e-38", "1e-45", "1e-46", "1e-50", "1e40", "1e100", "1e308", "1e-308",
    "3.4028235e38", "3.4028236e38", "3.40282347e38", "3.40282357e38",
    "1.17549435e-38", "1.4e-45", "7e-46", "7.006492e-46",
    "1e", "1e+", "1e-", "1ee", "1e5e5", ".e5", "e5", "E5",
    "0e0", "0e999999999999999999999", "0e-999999999999999999999",
    "1e999999999999999999999", "1e-999999999999999999999",
    "1e2147483647", "1e-2147483648", "1e4294967296",
    # --- hex floats ---
    "0x", "0X", "0x0", "0x1", "0X1", "0xf", "0xF", "0x10", "0xff",
    "0x1p0", "0x1p1", "0x1p-1", "0x1P4", "0x1p+4", "0x1p", "0x1p+", "0x1p-",
    "0x1.8p1", "0x.8p1", "0x1.p1", "0x.p1", "0x1.8", "0x1.8p2",
    "0x1.fffffep127", "0x1.ffffffp127", "0x1p128", "0x1p-149", "0x1p-150",
    "0x1p-126", "0x1p-127", "0x0.000002p-126",
    "0xg", "0x.g", "0xz", "0x1g", "0x1.8g",
    "0x123456789abcdef0123456789abcdef", "0x1.23456789abcdefp10",
    "0xfffffffffffffffffffffffffffffffp0",
    "-0x1p1", "+0x1p1", "-0x", "-0x0",
    # --- inf / nan ---
    "inf", "INF", "Inf", "iNf", "-inf", "+inf",
    "infinity", "INFINITY", "InFiNiTy", "-infinity",
    "in", "i", "inf ", "infi", "infin", "infini", "infinit",
    "infinityx", "infx", "inf1",
    "nan", "NAN", "NaN", "-nan", "+nan", "na", "n", "nax", "nanx",
    "nan(", "nan()", "nan(1)", "nan(123)", "nan(0x7f)", "-nan(5)",
    # --- signs / junk ---
    "-", "+", "--1", "++1", "-+1", "+-1", "- 1", "+ 1",
    "abc", "x", ".", "..", ".-5", "1.2.3", "1,5", "1 2", "1\n2",
    "\x00", "\x001", "1\x002",
    # --- leading whitespace then value ---
    "   1.5", "\n\n1.5", "\t-2.5e3", "\r\n\v\f 0x1p3", " \n inf",
    # --- trailing junk ---
    "1.5abc", "1.5 ", "1.5\n", "1.5\n\n", "0x1p3xyz", "inf\ninf",
    # --- rounding / ties ---
    "1.0000000596046448", "0.5000000000000001", "8388609.5", "8388608.5",
    "8388610.5", "16777215.5", "1.00000005960464477539062",
    "0.99999999999999999999", "1.99999999999999999999",
    "2.00000000000000000001", "1.5000000000000000001",
    # --- subnormal boundaries ---
    "1.1754942106924411e-38", "5.877471754111438e-39", "1.401298464324817e-45",
    "7.0064923216240854e-46", "7.0064923216240855e-46",
    "2.938735877055719e-39", "0.000000000000000000000000000000000000000000001",
    # --- overflow boundaries ---
    "340282346638528859811704183484516925440",
    "340282356779733661637539395458142568448",
    "340282366920938463463374607431768211456",
    "-340282366920938463463374607431768211456",
    # --- very long ---
    "1" + "0" * 500, "0." + "0" * 500 + "1", "1." + "2" * 1000,
    "0" * 500 + "1", "1e" + "9" * 100, "0x1." + "f" * 200 + "p0",
]


def rand_cases(seed=0xC0FFEE, n=4000):
    rng = random.Random(seed)
    out = []
    digits = "0123456789"
    hexd = "0123456789abcdefABCDEF"
    junk = "+-.eEpPxX inf" + digits

    for _ in range(n // 8):
        # random float via exact bit pattern round-trip
        bits = rng.getrandbits(32)
        f = struct.unpack("<f", struct.pack("<I", bits))[0]
        out.append(repr(f))
        out.append("%.17g" % f)
        out.append("%a" % f if f == f else "nan")

    for _ in range(n // 8):
        # random decimal literal
        ip = "".join(rng.choice(digits) for _ in range(rng.randint(0, 12)))
        fp = "".join(rng.choice(digits) for _ in range(rng.randint(0, 25)))
        s = rng.choice(["", "+", "-"]) + ip
        if rng.random() < 0.7:
            s += "." + fp
        if rng.random() < 0.6:
            s += rng.choice("eE") + rng.choice(["", "+", "-"]) + \
                 "".join(rng.choice(digits) for _ in range(rng.randint(0, 4)))
        out.append(s)

    for _ in range(n // 8):
        # random hex literal
        ip = "".join(rng.choice(hexd) for _ in range(rng.randint(0, 18)))
        fp = "".join(rng.choice(hexd) for _ in range(rng.randint(0, 18)))
        s = rng.choice(["", "+", "-"]) + rng.choice(["0x", "0X"]) + ip
        if rng.random() < 0.7:
            s += "." + fp
        if rng.random() < 0.7:
            s += rng.choice("pP") + rng.choice(["", "+", "-"]) + \
                 str(rng.randint(-200, 200))
        out.append(s)

    for _ in range(n // 8):
        # random junk soup
        s = "".join(rng.choice(junk) for _ in range(rng.randint(1, 14)))
        out.append(s)

    for _ in range(n // 8):
        # leading whitespace + value + trailing junk
        ws = "".join(rng.choice(" \t\n\r\v\f") for _ in range(rng.randint(0, 4)))
        val = rng.choice(FIXED)
        tail = "".join(rng.choice(junk) for _ in range(rng.randint(0, 4)))
        out.append(ws + val + tail)

    for _ in range(n // 8):
        # extreme exponents around the f32 boundaries
        mant = str(rng.randint(1, 10 ** rng.randint(1, 20)))
        e = rng.randint(-60, 60)
        out.append(mant + "e" + str(e))

    for _ in range(n // 8):
        # near-tie decimal strings (many digits)
        d = "".join(rng.choice(digits) for _ in range(rng.randint(20, 60)))
        out.append("0." + d + "e" + str(rng.randint(-50, 50)))

    for _ in range(n // 8):
        # hex mantissa long enough to trigger the sticky-bit path
        h = "".join(rng.choice(hexd) for _ in range(rng.randint(14, 40)))
        out.append("0x" + h + "p" + str(rng.randint(-160, 160)))

    return out


def all_cases(seed=0xC0FFEE, n=4000):
    seen = set()
    res = []
    for s in FIXED + rand_cases(seed, n):
        b = s.encode("utf-8", "surrogateescape")
        if b in seen:
            continue
        seen.add(b)
        res.append((b, s))
    return res


if __name__ == "__main__":
    cs = all_cases()
    print(len(cs))
