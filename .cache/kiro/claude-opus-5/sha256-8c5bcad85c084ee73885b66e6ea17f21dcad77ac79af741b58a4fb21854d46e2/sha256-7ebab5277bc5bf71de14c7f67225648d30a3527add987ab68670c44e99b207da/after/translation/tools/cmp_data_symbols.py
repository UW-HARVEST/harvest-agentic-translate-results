#!/usr/bin/env python3
"""Compare the contents of every exported data symbol between the C-built
libpcre2.so and the Rust-built one, byte for byte.

Both are ELF shared objects, so the symbol table gives the address and size of
each object; the bytes are read from the file at the corresponding offset.
"""
import subprocess
import sys

C_LIB = '/tmp/cb3tiS/libpcre2.so'
R_LIB = 'target/release/libpcre2.so'


def sections(path):
    out = subprocess.run(['readelf', '-SW', path], capture_output=True,
                         text=True, check=True).stdout
    secs = []
    for line in out.split('\n'):
        line = line.strip()
        if not line.startswith('['):
            continue
        rest = line.split(']', 1)
        if len(rest) != 2:
            continue
        f = rest[1].split()
        if len(f) < 5:
            continue
        name, typ, addr, off, size = f[0], f[1], f[2], f[3], f[4]
        try:
            secs.append((name, int(addr, 16), int(off, 16), int(size, 16)))
        except ValueError:
            continue
    return secs


def symbols(path):
    out = subprocess.run(['readelf', '-sW', '--dyn-syms', path],
                         capture_output=True, text=True, check=True).stdout
    syms = {}
    for line in out.split('\n'):
        f = line.split()
        if len(f) < 8 or not f[0].endswith(':'):
            continue
        try:
            value = int(f[1], 16)
            size = int(f[2])
        except ValueError:
            continue
        typ, name = f[3], f[7]
        if typ != 'OBJECT' or size == 0:
            continue
        syms[name.split('@')[0]] = (value, size)
    return syms


def reader(path):
    data = open(path, 'rb').read()
    secs = sections(path)

    def read(addr, size):
        for _name, saddr, soff, ssize in secs:
            if saddr != 0 and saddr <= addr < saddr + ssize:
                start = soff + (addr - saddr)
                return data[start:start + size]
        return None
    return read


csyms, rsyms = symbols(C_LIB), symbols(R_LIB)
cread, rread = reader(C_LIB), reader(R_LIB)

bad = ok = skipped = 0
for name in sorted(csyms):
    caddr, csize = csyms[name]
    if name not in rsyms:
        print(f"MISSING in Rust: {name}")
        bad += 1
        continue
    raddr, rsize = rsyms[name]
    cb, rb = cread(caddr, csize), rread(raddr, rsize)
    if cb is None or rb is None:
        skipped += 1
        continue
    if csize != rsize:
        print(f"SIZE  {name}: C={csize} Rust={rsize}")
        bad += 1
        continue
    if cb != rb:
        diffs = [i for i in range(csize) if cb[i] != rb[i]]
        print(f"BYTES {name}: {len(diffs)}/{csize} differ, first at {diffs[0]} "
              f"(C={cb[diffs[0]]:#x} Rust={rb[diffs[0]]:#x})")
        bad += 1
        continue
    ok += 1

print(f"\n{ok} identical, {bad} differing, {skipped} unreadable", file=sys.stderr)
sys.exit(1 if bad else 0)
