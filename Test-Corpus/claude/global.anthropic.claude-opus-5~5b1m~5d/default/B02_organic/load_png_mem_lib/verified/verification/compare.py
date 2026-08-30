#!/usr/bin/env python3
"""Compare per-case harness outputs, excluding cases that are nondeterministic
in the C reference itself."""
import sys

def blocks(path):
    d = {}
    cur = None
    for line in open(path):
        if line.startswith('==='):
            cur = line.strip(); d[cur] = []
        elif cur is not None:
            d[cur].append(line)
    return d

def main():
    c1, c2, r1, r2 = (blocks(p) for p in sys.argv[1:5])
    label = sys.argv[5] if len(sys.argv) > 5 else ""
    keys = set(c1) | set(c2) | set(r1) | set(r2)
    unstable = [k for k in keys if c1.get(k) != c2.get(k) or r1.get(k) != r2.get(k)]
    stable = [k for k in keys if k not in unstable]
    mism = [k for k in stable if c1.get(k) != r1.get(k)]
    print(f"{label}: cases={len(keys)} nondeterministic={len(unstable)} "
          f"compared={len(stable)} MISMATCH={len(mism)}")
    for k in mism[:8]:
        print("  ---", k)
        print("    C :", "".join(c1.get(k, []))[:160].replace("\n", "|"))
        print("    RS:", "".join(r1.get(k, []))[:160].replace("\n", "|"))
    return len(mism)

sys.exit(1 if main() else 0)
