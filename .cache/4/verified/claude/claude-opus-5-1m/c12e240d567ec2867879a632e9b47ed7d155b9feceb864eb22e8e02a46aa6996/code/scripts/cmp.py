#!/usr/bin/env python3
"""Loads both shared objects and compares one function over given arguments.

usage: cmp.py <func> <args...>      (floats may be given as 0x-hex bit patterns
                                     with a leading 'b', e.g. b0x7fc00000)
"""
import ctypes
import struct
import sys

C_SO = "target/cdiff/libc_driver.so"
RUST_SO = "target/debug/libstb_perlin_cli.so"

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
CT = {"f": ctypes.c_float, "i": ctypes.c_int, "B": ctypes.c_ubyte}


def parse(spec, raw):
    if spec != "f":
        return int(raw)
    if raw.startswith("b"):
        return struct.unpack("<f", struct.pack("<I", int(raw[1:], 16)))[0]
    return float(raw)


def get(so, name, params):
    lib = ctypes.CDLL(so)
    fn = getattr(lib, name)
    fn.restype = ctypes.c_float
    fn.argtypes = [CT[p] for p in params]
    return fn


def bits(v):
    return struct.unpack("<I", struct.pack("<f", v))[0]


def main():
    name = sys.argv[1]
    params = SIGS[name]
    args = [parse(s, r) for s, r in zip(params, sys.argv[2:])]
    cv = get(C_SO, name, params)(*args)
    rv = get(RUST_SO, name, params)(*args)
    flag = "OK " if bits(cv) == bits(rv) else "DIFF"
    print(f"{flag} {name}{tuple(sys.argv[2:])}: C=0x{bits(cv):08x} Rust=0x{bits(rv):08x}")
    return 0 if flag == "OK " else 1


if __name__ == "__main__":
    sys.exit(main())
