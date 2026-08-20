#!/usr/bin/env python3
"""Map which garbage bytes just past the initialised part of input_buffer /
ref_buffer in the C driver's `main` frame are zero.

Probe (operation 4, flags 0 -> case_insensitive path):
  input = k bytes of 'A', ref = k bytes of 'a' + NUL   -> result 6 iff input_buffer[k] == 0
  input = k bytes of 'A' + NUL, ref = k bytes of 'a'   -> result 6 iff ref_buffer[k]   == 0
"""
import subprocess
import sys

DRIVER = sys.argv[1] if len(sys.argv) > 1 else "c_src/build/driver"
MAXK = int(sys.argv[2]) if len(sys.argv) > 2 else 1024


def run(tokens):
    p = subprocess.run([DRIVER], input=" ".join(str(t) for t in tokens),
                       capture_output=True, text=True)
    return p.returncode, p.stdout.strip(), p.stderr.strip()


def probe_input(k):
    toks = [4, 0, k] + [65] * k + [k + 1] + [97] * k + [0]
    return run(toks)


def probe_ref(k):
    toks = [4, 0, k + 1] + [65] * k + [0] + [k] + [97] * k
    return run(toks)


def bitmap(fn, maxk):
    out = []
    for k in range(0, maxk + 1):
        rc, so, se = fn(k)
        if rc != 0:
            out.append("!")
        elif so == "6":
            out.append("0")   # byte is zero
        elif so == "1" and k == 0:
            out.append("0")
        elif so == "0":
            out.append("X")   # byte is non-zero
        else:
            out.append("?" + so)
    return out


for name, fn in (("input_buffer", probe_input), ("ref_buffer", probe_ref)):
    bm = bitmap(fn, MAXK)
    print(f"== {name} (0=zero byte, X=non-zero, ?=other result) ==")
    for base in range(0, len(bm), 64):
        chunk = bm[base:base + 64]
        print(f"{base:5d}: " + "".join(chunk))
    print()
