#!/usr/bin/env python3
import subprocess, sys, os, itertools
BASE = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
C = os.path.join(BASE, "c_src/build/driver")
R = os.path.join(BASE, "translation/target/release/driver")

def run(exe, data):
    p = subprocess.run([exe], input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return p.stdout, p.stderr, p.returncode

toks = []
# hex subnormal / double-rounding candidates (must fit in 19 chars)
toks += [b"0x3p-1076", b"0x1p-1074", b"0x1p-1075", b"0x1p-1076", b"0x7p-1077",
         b"0x1p-1022", b"0x1p1024", b"0x1p1023", b"0xfffffffffffffp0",
         b"0x1fffffffffffffp0", b"0x1.fffffffffffffp0", b"0x1.fffffffffffff8p0",
         b"0x10000000000001p0", b"0x1p+1000", b"0x0p0", b"0x0.0p0", b"0x.p0",
         b"0x1p999999999", b"0x1p-999999999", b"0x1p2147483648", b"0x1p99999999999"]
# decimal edge cases within 19 chars
toks += [b"1e-323", b"5e-324", b"2.5e-324", b"1.5e-323", b"4.9e-324", b"1e-320",
         b"1.7976931348e308", b"1e308", b"1e309", b"9.9999999999999e99",
         b"0.1", b"1e-45", b"1.4e-45", b"7.0e-46", b"7.1e-46", b"3.4028236e38",
         b"3.4028234e38", b"3.4028235e38", b"1.9999999e-6", b"1.0000000e-6",
         b"9.9999995e-7", b"9.9999994e-7", b"1.00000001e-6", b"0.9999999e-6"]
# 100/x near INT_MAX / INT_MIN boundaries
for m in [b"4.6566127e-8", b"4.6566129e-8", b"4.656613e-8", b"4.65661e-8",
          b"-4.6566127e-8", b"-4.6566129e-8", b"4.6566128731e-8"]:
    toks.append(m)
# whitespace / sign / partial-keyword permutations
toks += [b"i", b"in", b"inf ", b"infin", b"infinit", b"infinity1", b"n", b"na",
         b"nan(", b"nan()", b"NAN(_)", b"+nan", b"+inf", b"- 5", b"+ 5",
         b"--5", b"++5", b"5e5e5", b"5..5", b".e5", b"0e", b"0x1p1p1",
         b"\x00", b"\x005", b"    ", b"\t\t\t", b"\n", b"e", b"E", b".", b"-.",
         b"0", b"-0", b"+0", b"0.0e999", b"0e999", b"0e-999", b"-0e5"]

CASES = []
for t in toks:
    CASES.append((t.decode("latin1"), t + b"\n" + t + b"\n"))
    CASES.append((t.decode("latin1") + "|noeol", t))

# fgets boundary: lines of every length 0..25 (splitting across the 20-byte buffer)
for n in range(0, 26):
    body = b"1" * n
    CASES.append(("len%d" % n, body + b"\n2\n"))
    CASES.append(("len%d_noeol" % n, body))

fails = 0
for name, data in CASES:
    a, b = run(C, data), run(R, data)
    if a != b:
        fails += 1
        print("FAIL %s input=%r\n   C: %r\n   R: %r" % (name, data, a, b))
print("%d/%d failed" % (fails, len(CASES)))
sys.exit(1 if fails else 0)
