#!/usr/bin/env python3
"""Compare the C library's file-static tables against the Rust equivalents that
`src/dbg_tables.rs` temporarily exports as `DBG_<name>`."""
import subprocess
import sys

C_LIB = '/tmp/cb3tiS/libpcre2.so'
R_LIB = 'target/release/libpcre2.so'

PAIRS = [
    'escapes', 'xdigitab', 'meta_extra_lengths', 'opcode_possessify',
    'chartypeoffset', 'verbnames', 'verbops', 'alasnames', 'posix_names',
    'posix_substitutes', 'verbs', 'alasmeta',
    'autoposstab', 'catposstab', 'propposstab', 'posspropstab',
    'coptable', 'poptable', 'toptable1', 'toptable2',
    'rep_min', 'rep_max', 'rep_typ',
]
# posix_name_lengths and posix_class_maps have no size-bearing C symbol in some
# builds; they are compared when present.
OPTIONAL = ['posix_name_lengths', 'posix_class_maps']


def sections(path):
    out = subprocess.run(['readelf', '-SW', path], capture_output=True,
                         text=True, check=True).stdout
    secs = []
    for line in out.split('\n'):
        line = line.strip()
        if not line.startswith('['):
            continue
        parts = line.split(']', 1)
        if len(parts) != 2:
            continue
        f = parts[1].split()
        if len(f) < 5:
            continue
        try:
            secs.append((int(f[2], 16), int(f[3], 16), int(f[4], 16)))
        except ValueError:
            continue
    return secs


def symbols(path):
    out = subprocess.run(['readelf', '-sW', path], capture_output=True,
                         text=True, check=True).stdout
    syms = {}
    for line in out.split('\n'):
        f = line.split()
        if len(f) < 8 or not f[0].endswith(':'):
            continue
        try:
            value, size = int(f[1], 16), int(f[2])
        except ValueError:
            continue
        if f[3] != 'OBJECT' or size == 0:
            continue
        syms[f[7].split('@')[0]] = (value, size)
    return syms


def reader(path):
    data = open(path, 'rb').read()
    secs = sections(path)

    def read(addr, size):
        for saddr, soff, ssize in secs:
            if saddr != 0 and saddr <= addr < saddr + ssize:
                start = soff + (addr - saddr)
                return data[start:start + size]
        return None
    return read


csyms, rsyms = symbols(C_LIB), symbols(R_LIB)
cread, rread = reader(C_LIB), reader(R_LIB)

bad = 0
for name in PAIRS + OPTIONAL:
    dbg = 'DBG_' + name
    if name not in csyms:
        if name not in OPTIONAL:
            print(f"{name}: no C symbol")
            bad += 1
        continue
    if dbg not in rsyms:
        print(f"{name}: no Rust symbol {dbg}")
        bad += 1
        continue
    caddr, csize = csyms[name]
    raddr, rsize = rsyms[dbg]
    cb, rb = cread(caddr, csize), rread(raddr, rsize)
    if csize != rsize:
        print(f"{name}: SIZE C={csize} Rust={rsize}")
        bad += 1
        continue
    if cb != rb:
        d = [i for i in range(csize) if cb[i] != rb[i]]
        print(f"{name}: {len(d)}/{csize} bytes differ, first at {d[0]} "
              f"(C={cb[d[0]]:#x} Rust={rb[d[0]]:#x})")
        bad += 1
        continue
    print(f"{name}: identical ({csize} bytes)")

sys.exit(1 if bad else 0)
