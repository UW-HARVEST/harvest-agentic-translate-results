#!/usr/bin/env python3
"""Audit which exported symbols the differential tests actually reach.

Two kinds of reference are recognised:

1. **literal** — the symbol name appears verbatim in a `tests/*.rs` file.
2. **composed** — the test builds the name at run time, e.g.
   `format!("{p}_encrypt")` with `p` drawn from a list of prefixes. We detect
   this by collecting every string literal in the test sources, deriving the
   set of `format!` suffix patterns (`_encrypt`, `_keybytes`, ...), and pairing
   them with every prefix literal that appears in the same file.

Symbols matched by neither are reported as UNREFERENCED and need attention.
"""
import glob
import re
import subprocess
import sys

SKIP = {
    "_init", "_fini", "_edata", "_end", "__bss_start", "_IO_stdin_used",
    "__gmon_start__", "_ITM_deregisterTMCloneTable", "_ITM_registerTMCloneTable",
}


def exported(path):
    out = subprocess.check_output(["nm", "-D", "--defined-only", path], text=True)
    syms = set()
    for line in out.splitlines():
        p = line.split()
        if len(p) >= 3 and p[2] not in SKIP:
            syms.add(p[2])
    return syms


def main():
    syms = exported("c_src/build/libsodium.so")
    files = sorted(glob.glob("tests/*.rs")) + sorted(glob.glob("tests/common/*.rs"))
    per_file = {f: open(f, errors="replace").read() for f in files}
    all_src = "\n".join(per_file.values())

    literal = {s for s in syms if s in all_src}

    # composed: format!("{x}_suffix") / format!("{}_suffix", x) patterns
    suffixes = set()
    for m in re.finditer(r'format!\(\s*"([^"]*)"', all_src):
        t = m.group(1)
        # keep the part after the last closing brace
        if "}" in t:
            tail = t[t.rindex("}") + 1:]
            if tail.startswith("_") and re.fullmatch(r"[a-z0-9_]+", tail):
                suffixes.add(tail)
        # and "{p}_a_{q}" style middles
    # prefixes: every string literal that is a prefix of some exported symbol
    prefixes = set()
    for m in re.finditer(r'"([a-z_][a-z0-9_]{3,})"', all_src):
        v = m.group(1)
        if any(s.startswith(v) for s in syms):
            prefixes.add(v)

    composed = set()
    for p in prefixes:
        for suf in suffixes:
            cand = p + suf
            if cand in syms:
                composed.add(cand)
        if p in syms:
            composed.add(p)

    reached = literal | composed
    missing = sorted(syms - reached)

    print(f"exported symbols (excl. ELF runtime): {len(syms)}")
    print(f"  referenced literally in tests      : {len(literal)}")
    print(f"  reachable via format! composition  : {len(composed - literal)}")
    print(f"  TOTAL reached                      : {len(reached)}")
    print(f"  UNREFERENCED                       : {len(missing)}")
    if missing:
        print("\nUNREFERENCED SYMBOLS:")
        for m in missing:
            print(f"  {m}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
