#!/usr/bin/env python3
"""Differential test harness: C executable vs Rust executable.

Feeds identical bytes on stdin to both `c_src/build/driver` and
`target/release/driver` and compares stdout byte-for-byte.
"""
import subprocess
import sys
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
C_BIN = os.path.join(ROOT, "c_src", "build", "driver")
R_BIN = os.path.join(ROOT, "target", "release", "driver")


def run(binary, data: bytes):
    p = subprocess.run([binary], input=data, stdout=subprocess.PIPE,
                       stderr=subprocess.PIPE, timeout=20)
    return p.returncode, p.stdout, p.stderr


def compare(data: bytes, label=None):
    """Returns None if identical, else a description string."""
    crc, cout, _ = run(C_BIN, data)
    rrc, rout, _ = run(R_BIN, data)
    if cout != rout or crc != rrc:
        return (f"INPUT {label or data!r}\n"
                f"  C:    rc={crc} out={cout!r}\n"
                f"  RUST: rc={rrc} out={rout!r}")
    return None


def main(cases):
    fails = []
    for data, label in cases:
        d = compare(data, label)
        if d:
            fails.append(d)
    print(f"ran {len(cases)} cases, {len(fails)} failures")
    for f in fails[:80]:
        print(f)
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(0)
