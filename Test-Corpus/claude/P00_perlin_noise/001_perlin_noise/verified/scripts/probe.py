#!/usr/bin/env python3
"""Quick ctypes probe of one C entry point; used to explore the C behaviour
(including the undefined-behaviour corners) before writing the Rust tests.

usage: probe.py <so> <func> <args...>   -- prints the float bits, or crashes
"""
import ctypes
import struct
import sys

SIGS = {
    "stb_perlin_noise3_internal": ("f", ["f", "f", "f", "i", "i", "i", "B"]),
    "stb_perlin_noise3": ("f", ["f", "f", "f", "i", "i", "i"]),
    "stb_perlin_noise3_seed": ("f", ["f", "f", "f", "i", "i", "i", "i"]),
    "stb_perlin_ridge_noise3": ("f", ["f", "f", "f", "f", "f", "f", "i"]),
    "stb_perlin_fbm_noise3": ("f", ["f", "f", "f", "f", "f", "i"]),
    "stb_perlin_turbulence_noise3": ("f", ["f", "f", "f", "f", "f", "i"]),
    "stb_perlin_noise3_wrap_nonpow2": ("f", ["f", "f", "f", "i", "i", "i", "B"]),
    "inner": ("f", ["i", "f", "f", "f", "i", "i", "i", "i", "f", "f", "f", "i"]),
}

CT = {"f": ctypes.c_float, "i": ctypes.c_int, "B": ctypes.c_ubyte}


def main() -> int:
    so, name = sys.argv[1], sys.argv[2]
    ret, params = SIGS[name]
    lib = ctypes.CDLL(so)
    fn = getattr(lib, name)
    fn.restype = CT[ret]
    fn.argtypes = [CT[p] for p in params]
    args = []
    for spec, raw in zip(params, sys.argv[3:]):
        if spec == "f":
            args.append(float.fromhex(raw) if raw.startswith(("0x", "-0x")) else float(raw))
        else:
            args.append(int(raw))
    value = fn(*args)
    bits = struct.unpack("<I", struct.pack("<f", value))[0]
    print(f"{value!r} 0x{bits:08x}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
