"""Exact ties-to-even probing: dyadic midpoints rendered exactly in decimal
and in hex, plus one unit-in-last-place either side."""
import os, struct, subprocess, random, sys
from decimal import Decimal, getcontext
from concurrent.futures import ThreadPoolExecutor
getcontext().prec = 400
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
C_BIN = os.path.join(ROOT, "c_src", "build", "driver")
R_BIN = os.path.join(ROOT, "target", "release", "driver")

def one(data):
    c = subprocess.run([C_BIN], input=data, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    r = subprocess.run([R_BIN], input=data, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    if c.stdout != r.stdout:
        return f"{data!r}: C={c.stdout!r} RUST={r.stdout!r}"
    return None

def f32(bits):
    return struct.unpack("<f", struct.pack("<I", bits))[0]

def exact_dec(x):
    """Exact decimal expansion of a python float (dyadic rational)."""
    return Decimal(x)

rng = random.Random(int(sys.argv[1]) if len(sys.argv) > 1 else 7)
cases = []
N = int(sys.argv[2]) if len(sys.argv) > 2 else 1500

for _ in range(N):
    bits = rng.getrandbits(31)          # positive, any exponent
    e = (bits >> 23) & 0xff
    if e >= 0xfe:
        continue
    a = f32(bits)
    b = f32(bits + 1)
    mid = (Decimal(a) + Decimal(b)) / 2   # exact: dyadic / 2
    s = format(mid, 'f')
    cases.append(s.encode())
    cases.append(("-" + s).encode())
    # one ulp of the decimal string either side
    if '.' in s:
        digits = s.replace('.', '').lstrip('0')
        cases.append((s + "1").encode())          # strictly above the midpoint
        cases.append((s[:-1] + str(int(s[-1]) - 1) + "9" * 30).encode() if s[-1] != '0' else s.encode())
    # scientific rendering of the same exact value
    cases.append(format(mid, 'e').encode())
    # exact hex rendering of the midpoint (a is dyadic, midpoint = a + 2^-k)
    cases.append(float.hex((a + b) / 2 if (a + b) / 2 != 0 else a).encode())

# also exact expansions of the f32 values themselves
for _ in range(N):
    bits = rng.getrandbits(32)
    v = f32(bits)
    if v != v or v in (float('inf'), float('-inf')):
        continue
    cases.append(format(Decimal(v), 'f').encode())
    cases.append(float.hex(v).encode())

seen = set(); uniq = []
for c in cases:
    if c not in seen:
        seen.add(c); uniq.append(c)
print(f"cases={len(uniq)}", flush=True)
fails = []
with ThreadPoolExecutor(max_workers=48) as ex:
    for res in ex.map(one, uniq):
        if res: fails.append(res)
print(f"failures={len(fails)}")
for f in fails[:40]: print(f)
