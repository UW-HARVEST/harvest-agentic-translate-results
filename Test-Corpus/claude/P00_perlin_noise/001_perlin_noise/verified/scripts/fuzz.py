#!/usr/bin/env python3
"""Large randomised differential sweep of the C and Rust shared objects.

This is the "more of everything" companion of the cargo tests: it drives all
eight exported functions through ctypes (fast, in-process) and additionally
compares the two driver executables on random stdin text.

usage: fuzz.py [iterations-per-function] [driver-iterations]
"""
import ctypes
import random
import re
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
C_SO = ROOT / "target/cdiff/libc_driver.so"
C_EXE = ROOT / "target/cdiff/c_driver"
RUST_SO = ROOT / "target/debug/libstb_perlin_cli.so"
RUST_SO_ALT = ROOT / "target/sotest/debug/libstb_perlin_cli.so"
RUST_EXE = ROOT / "target/debug/driver"

CT = {"f": ctypes.c_float, "i": ctypes.c_int, "B": ctypes.c_ubyte}
SIGS = {
    "stb_perlin_noise3_internal": ["f", "f", "f", "i", "i", "i", "B"],
    "stb_perlin_noise3": ["f", "f", "f", "i", "i", "i"],
    "stb_perlin_noise3_seed": ["f", "f", "f", "i", "i", "i", "i"],
    "stb_perlin_ridge_noise3": ["f", "f", "f", "f", "f", "f", "i"],
    "stb_perlin_fbm_noise3": ["f", "f", "f", "f", "f", "i"],
    "stb_perlin_turbulence_noise3": ["f", "f", "f", "f", "f", "i"],
    "stb_perlin_noise3_wrap_nonpow2": ["f", "f", "f", "i", "i", "i", "B"],
    "inner": ["i", "f", "f", "f", "i", "i", "i", "i", "f", "f", "f", "i"],
}


def load(path):
    lib = ctypes.CDLL(str(path))
    out = {}
    for name, params in SIGS.items():
        fn = getattr(lib, name)
        fn.restype = ctypes.c_float
        fn.argtypes = [CT[p] for p in params]
        out[name] = fn
    return out


def bits(v):
    return struct.unpack("<I", struct.pack("<f", v))[0]


def f32(b):
    return struct.unpack("<f", struct.pack("<I", b))[0]


# --- the C permutation tables, parsed out of the header ---------------------
def table_window():
    text = (ROOT / "c_src/src/stb_perlin.h").read_text()

    def grab(decl):
        start = text.index(decl) + len(decl)
        body = text[start : text.index("}", start)]
        vals = [int(t) for t in re.findall(r"\d+", re.sub(r"//[^\n]*", "", body))]
        assert len(vals) >= 512, decl
        return vals[:512]

    return grab("stb__perlin_randtab[512] =") + grab("stb__perlin_randtab_grad_idx[512] =")


WINDOW = table_window()
INT_MIN, INT_MAX = -(2**31), 2**31 - 1


def to_int32(v):
    v &= 0xFFFFFFFF
    return v - 2**32 if v >= 2**31 else v


def fastfloor(a):
    if a != a or not (-2147483648.0 <= a < 2147483648.0):
        ai = INT_MIN
    else:
        ai = int(a)
    return to_int32(ai - 1) if a < float(ai) else ai


def c_rem(a, b):
    """C's % (truncated toward zero), with wrapping i32 arithmetic."""
    if b == 0:
        return 0
    q = abs(a) // abs(b)
    if (a < 0) != (b < 0):
        q = -q
    return to_int32(a - to_int32(q * b))


def reproducible(x, y, z, xw, yw, zw, seed):
    """True when every table index of wrap_nonpow2 stays in the 1024 byte window."""
    px, py, pz = fastfloor(x), fastfloor(y), fastfloor(z)
    w = [xw or 256, yw or 256, zw or 256]
    for p, wr in zip((px, py, pz), w):
        if p == INT_MIN and wr == -1:
            return False  # SIGFPE
    idx = []
    for p, wr in zip((px, py, pz), w):
        a0 = c_rem(p, wr)
        if a0 < 0:
            a0 = to_int32(a0 + wr)
        a1 = c_rem(to_int32(a0 + 1), wr)
        idx.append((a0, a1))
    ok = True

    def rt(i):
        nonlocal ok
        if 0 <= i < 1024:
            return WINDOW[i]
        ok = False
        return 0

    r0 = rt(rt(idx[0][0]) + seed)
    r1 = rt(rt(idx[0][1]) + seed)
    rr = [rt(r0 + idx[1][0]), rt(r0 + idx[1][1]), rt(r1 + idx[1][0]), rt(r1 + idx[1][1])]
    for r in rr:
        for zz in idx[2]:
            if not 0 <= r + zz < 512:
                ok = False
    return ok


# --- generators -------------------------------------------------------------
SPECIAL = [0.0, -0.0, 1.0, -1.0, 0.5, float("inf"), float("-inf"), float("nan"),
           -float("nan"), 3.4028234663852886e38, -3.4028234663852886e38, 1e-45, 1e30, -1e30,
           16777216.0, 2147483520.0, -2147483648.0, 4294967296.0]
WRAPS = [0, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 3, 5, 7, 100, 255, 257, -1, -2, -5,
         -256, INT_MAX, INT_MIN]


def rand_float(rng):
    k = rng.randrange(5)
    if k == 0:
        return rng.choice(SPECIAL)
    if k == 1:
        return f32(rng.getrandbits(32))
    if k == 2:
        return float(rng.randrange(-1000, 1000)) + rng.choice([0.0, 0.5, 0.25, 0.125])
    if k == 3:
        return rng.uniform(-8, 8)
    return f32(rng.choice([0x7FC00000, 0xFFC00000, 0x7F800001, 0x00000001, 0x807FFFFF]))


def rand_wrap(rng):
    k = rng.randrange(3)
    if k == 0:
        return rng.choice(WRAPS)
    if k == 1:
        return rng.randrange(-1100, 1100)
    return to_int32(rng.getrandbits(32))


def main():
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 200000
    driver_n = int(sys.argv[2]) if len(sys.argv) > 2 else 2000
    rust_so = RUST_SO if RUST_SO.exists() else RUST_SO_ALT
    c, r = load(C_SO), load(rust_so)
    rng = random.Random(0xD1FF)
    failures = 0
    counts = {}

    def cmp(name, args):
        nonlocal failures
        cv, rv = c[name](*args), r[name](*args)
        counts[name] = counts.get(name, 0) + 1
        if bits(cv) != bits(rv):
            failures += 1
            if failures < 20:
                print(f"DIFF {name}{args}: C=0x{bits(cv):08x} Rust=0x{bits(rv):08x}")

    for _ in range(n):
        x, y, z = rand_float(rng), rand_float(rng), rand_float(rng)
        xw, yw, zw = rand_wrap(rng), rand_wrap(rng), rand_wrap(rng)
        seed_i = to_int32(rng.getrandbits(32))
        seed_b = seed_i & 0xFF
        lac, gain, off = rand_float(rng), rand_float(rng), rand_float(rng)
        octaves = rng.choice([0, 1, 2, 3, 4, 6, 8, -1, -5, 17])
        cmp("stb_perlin_noise3_internal", (x, y, z, xw, yw, zw, seed_b))
        cmp("stb_perlin_noise3", (x, y, z, xw, yw, zw))
        cmp("stb_perlin_noise3_seed", (x, y, z, xw, yw, zw, seed_i))
        cmp("stb_perlin_ridge_noise3", (x, y, z, lac, gain, off, octaves))
        cmp("stb_perlin_fbm_noise3", (x, y, z, lac, gain, octaves))
        cmp("stb_perlin_turbulence_noise3", (x, y, z, lac, gain, octaves))
        if reproducible(x, y, z, xw, yw, zw, seed_b):
            cmp("stb_perlin_noise3_wrap_nonpow2", (x, y, z, xw, yw, zw, seed_b))
        which = rng.randrange(-2, 8)
        if which != 5 or reproducible(x, y, z, xw, yw, zw, seed_i & 0xFF):
            cmp("inner", (which, x, y, z, xw, yw, zw, seed_i, lac, gain, off, octaves))

    print("library calls compared:")
    for k, v in sorted(counts.items()):
        print(f"  {k:32} {v}")
    print(f"library divergences: {failures}")

    # --- driver executables -------------------------------------------------
    tokens = ["0", "1", "2", "3", "4", "5", "6", "-1", "0.5", "-0.5", "1e3", "1e", "1e-", ".5",
              "5.", "0x", "0x.", "0x1p3", "0x1.8p-2", "inf", "-inf", "nan", "-nan", "in", "na",
              "abc", "-", "+", ".", "2147483648", "-2147483649", "99999999999999999999", "007",
              "0x10", "1e400", "1e-400", "255", "256", "-256", "1.5", "2.25", "-13.5", "8", "6"]
    seps = [" ", "\t", "\n", "  ", "\r\n"]
    dfail = 0
    for i in range(driver_n):
        if i % 3 == 0:
            # well formed
            which = rng.randrange(0, 6)
            fields = [str(which)]
            fields += [repr(round(rng.uniform(-300, 300), 4)) for _ in range(3)]
            if which == 5:
                fields += [str(rng.randrange(0, 257)) for _ in range(3)]
            else:
                fields += [str(rng.choice(WRAPS)) for _ in range(3)]
            fields.append(str(to_int32(rng.getrandbits(32))))
            fields += [repr(round(rng.uniform(-4, 4), 4)) for _ in range(3)]
            fields.append(str(rng.randrange(0, 9)))
            text = " ".join(fields) + "\n"
        else:
            k = rng.randrange(0, 16)
            text = "".join(
                (rng.choice(seps) if j else "") + rng.choice(tokens) for j in range(k)
            )
            if rng.random() < 0.5:
                text += "\n"
        cout = subprocess.run([str(C_EXE)], input=text, capture_output=True, text=True)
        rout = subprocess.run([str(RUST_EXE)], input=text, capture_output=True, text=True)
        if (cout.stdout, cout.returncode) != (rout.stdout, rout.returncode):
            dfail += 1
            if dfail < 20:
                print(f"DIFF driver {text!r}: C={cout.stdout!r}/{cout.returncode} "
                      f"Rust={rout.stdout!r}/{rout.returncode}")
    print(f"driver inputs compared: {driver_n}, divergences: {dfail}")
    return 1 if failures or dfail else 0


if __name__ == "__main__":
    sys.exit(main())
