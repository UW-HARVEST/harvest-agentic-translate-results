"""Targeted fuzzing of the boundary constants in the Rust conversion:
  * decimal:  dp > 45 -> inf,  dp < -50 -> zero   (dp = exp10 + ndigits - lz)
  * hex:      mant>>59 sticky, e_val > 127, e_val < -200, shift >= 64
"""
import os, random, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
C_BIN = os.path.join(ROOT, "c_src", "build", "driver")
R_BIN = os.path.join(ROOT, "target", "release", "driver")

def one(data):
    c = subprocess.run([C_BIN], input=data, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    r = subprocess.run([R_BIN], input=data, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    if c.stdout != r.stdout:
        return f"{data[:120]!r}: C={c.stdout!r} RUST={r.stdout!r}"
    return None

rng = random.Random(int(sys.argv[1]) if len(sys.argv) > 1 else 1)
cases = []

# --- decimal dp boundary sweep -------------------------------------------
for lz in range(0, 12):                       # leading zeros in the digit run
    for nd in range(1, 12):                   # significant digits
        digits = "0" * lz + "".join(rng.choice("123456789") for _ in range(nd))
        for exp in range(-70, 70):
            cases.append(f"{digits}e{exp}".encode())
            cases.append(f"0.{digits}e{exp}".encode())
            cases.append(f"-{digits}e{exp}".encode())

# --- hex exponent boundary sweep -----------------------------------------
for nh in [1, 8, 14, 15, 16, 17, 20, 32]:
    mant = "".join(rng.choice("0123456789abcdef") for _ in range(nh))
    for exp in list(range(-220, -120)) + list(range(100, 140)):
        cases.append(f"0x{mant}p{exp}".encode())
        cases.append(f"-0x{mant}p{exp}".encode())
        cases.append(f"0x0.{mant}p{exp}".encode())
        cases.append(f"0x{mant}.{mant}p{exp}".encode())

# --- hex sticky-bit: 60-bit accumulator boundary --------------------------
for nh in range(12, 22):
    for _ in range(40):
        mant = rng.choice("123456789abcdef") + "".join(rng.choice("0123456789abcdef") for _ in range(nh - 1))
        cases.append(f"0x{mant}p{rng.randint(-30, 30)}".encode())
        # trailing zeros vs trailing nonzero: exactly what the sticky bit sees
        cases.append(f"0x{mant}{'0'*rng.randint(1,6)}p{rng.randint(-30,30)}".encode())
        cases.append(f"0x{mant}{'1'*rng.randint(1,6)}p{rng.randint(-30,30)}".encode())

# --- decimal with huge digit runs around the subnormal cliff --------------
for _ in range(3000):
    nd = rng.randint(1, 45)
    digits = "".join(rng.choice("0123456789") for _ in range(nd))
    cases.append(f"0.{'0'*rng.randint(0,50)}{digits}e{rng.randint(-60,60)}".encode())

seen=set(); uniq=[]
for c in cases:
    if c not in seen:
        seen.add(c); uniq.append(c)
print(f"cases={len(uniq)}", flush=True)
fails=[]
with ThreadPoolExecutor(max_workers=48) as ex:
    for res in ex.map(one, uniq):
        if res: fails.append(res)
print(f"failures={len(fails)}")
for f in fails[:50]: print(f)
