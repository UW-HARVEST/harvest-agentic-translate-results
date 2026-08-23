#!/usr/bin/env python3
"""Diff exported symbols between the reference C .so and the Rust cdylib."""
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
RUST_SO = os.path.join(HERE, "..", "target/release/libsodium.so")
C_SYMS = os.path.join(HERE, "c_symbols.txt")
MAP = os.path.join(HERE, "c_symbol_map.txt")


def defined(path):
    out = subprocess.run(
        ["nm", "-D", "--defined-only", path], capture_output=True, text=True
    ).stdout
    syms = set()
    for line in out.splitlines():
        p = line.split()
        if len(p) >= 3:
            syms.add(p[2])
    return syms


def main():
    c = set(open(C_SYMS).read().split())
    if not os.path.exists(RUST_SO):
        print("rust .so missing:", RUST_SO)
        sys.exit(1)
    r = defined(RUST_SO)
    missing = sorted(c - r)
    extra = sorted(s for s in (r - c) if not s.startswith("_"))

    # group missing by source file
    groups = {}
    cur = None
    for line in open(MAP):
        if line.startswith("###"):
            cur = line[4:].split("  (")[0].strip()
            groups[cur] = []
        elif line.strip():
            groups[cur].append(line.split()[1])

    print(f"C exports: {len(c)}   Rust exports: {len(r)}   MISSING: {len(missing)}")
    if missing:
        ms = set(missing)
        print("\n--- missing, by C source file ---")
        for src in sorted(groups):
            m = [s for s in groups[src] if s in ms]
            if m:
                print(f"{src}  ({len(m)}/{len(groups[src])} missing)")
                if "-v" in sys.argv:
                    for s in m:
                        print("    ", s)
    if extra:
        print(f"\n--- extra non-underscore Rust exports ({len(extra)}) ---")
        for s in extra:
            print("   ", s)
    sys.exit(1 if missing else 0)


main()
