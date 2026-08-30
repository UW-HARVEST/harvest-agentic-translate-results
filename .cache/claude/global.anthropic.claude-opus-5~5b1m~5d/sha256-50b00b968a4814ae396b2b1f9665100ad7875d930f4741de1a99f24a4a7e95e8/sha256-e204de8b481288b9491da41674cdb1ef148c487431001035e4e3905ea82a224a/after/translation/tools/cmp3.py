#!/usr/bin/env python3
import subprocess, sys, os, itertools, random
BASE = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
C = os.path.join(BASE, "c_src/build/driver")
R = os.path.join(BASE, "translation/target/release/driver")

def run(exe, data):
    p = subprocess.run([exe], input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return p.stdout, p.stderr, p.returncode

toks = [
    # "digits '.' exponent" -- does Rust's FromStr accept "5.e3"?
    b"5.e3", b"5.E3", b"5.e-3", b"50.e-1", b"0.e0", b"5.e", b".5e3", b".5e-3",
    b"5.e+3", b"0000.e3", b"1.e308", b"1.e-308",
    # gigantic exponent digit strings (fit in 19 chars)
    b"1e9999999999999999", b"1e-999999999999999", b"1e99999999", b"1e-99999999",
    b"1e2147483648", b"1e-2147483648", b"1e4294967296", b"0e9999999999999999",
    b"1.0e999999999999999", b"1e18446744073709551",
    # exponent with many leading zeros
    b"1e000000000000000002", b"1e00000000000000002", b"5e0000000000000000",
    # long mantissas of all 9s / rounding ties
    b"0.99999999999999999", b"1.9999999999999999", b"9999999999999999999",
    b"1.0000000000000002", b"1.0000000000000001", b"2.0000000000000004",
    # values that round differently in f32 vs f64
    b"16777217", b"16777216", b"1.00000005960464478",
    b"0.000001000000000001", b"0.0000009999999999999",
    # hex with p and long exponents
    b"0x1p99999999999999", b"0x1p-9999999999999", b"0x1.fp1000",
    b"0xabcdefp-20", b"0xABCDEFp-20", b"0x123456789abcdefp0",
    b"0x1000000000000000", b"0x.0000000000001p0",
    # infinity/nan mixed case & partial with more suffix
    b"InFiNiTy", b"iNfInItY", b"NaNq", b"infq", b"nanny", b"info",
    # signs and spaces inside
    b"  -  5", b"-  5", b" -5 ", b"\x0b-5", b"5\t", b"5\r", b"5\n",
    # results that are exactly INT_MIN / INT_MAX after 100/x
    b"-4.65661287307739e-8", b"4.65661287307739e-8",
    b"-4.65661287307740e-8", b"4.65661287307740e-8",
    b"-4.656612e-8", b"4.656612e-8",
    # float subnormal boundary tokens
    b"1.1754944e-38", b"1.1754942e-38", b"5.877472e-39", b"1.4012985e-45",
    b"7.0064923e-46", b"7.0064922e-46",
]
CASES = []
for t in toks:
    CASES.append((t, t + b"\n" + t + b"\n"))
    CASES.append((t, t))
    CASES.append((t, b"2\n" + t + b"\n"))

# exhaustive short-token sweep over a nasty alphabet (lengths 1..3)
AL = b"0.expni+-f a"
for n in (1, 2, 3):
    for combo in itertools.product(AL, repeat=n):
        t = bytes(combo)
        CASES.append((t, t + b"\n" + t + b"\n"))

fails = 0
for name, data in CASES:
    a, b = run(C, data), run(R, data)
    if a != b:
        fails += 1
        print("FAIL %r input=%r\n   C: %r\n   R: %r" % (name, data, a, b))
        if fails > 20:
            break
print("%d/%d failed" % (fails, len(CASES)))
sys.exit(1 if fails else 0)
