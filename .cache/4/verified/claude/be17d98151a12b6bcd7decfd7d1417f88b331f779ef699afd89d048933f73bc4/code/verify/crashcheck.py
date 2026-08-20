#!/usr/bin/env python3
"""For the cases that faulted, check C and Rust *separately* (own process each)
so we can tell whether both libraries fault the same way."""
import ctypes, os, subprocess, sys
import difftest as dt

IDX = [467, 477, 487, 617, 627, 637, 647, 777, 787, 797, 807, 837, 847, 877, 887,
       917, 927, 937, 947, 957]

def child(so, i):
    cases = dt.build_cases()
    data, out_size, in_off, out_off, label = cases[i]
    lib = dt.load(so)
    r = dt.run(lib, data, out_size, in_off, out_off)
    print(repr((r[0], r[2])))
    os._exit(0)

if __name__ == "__main__":
    if len(sys.argv) == 4:
        child(sys.argv[1], int(sys.argv[2]))
    c_so, r_so = sys.argv[1], sys.argv[2]
    cases = dt.build_cases()
    idx = IDX if len(sys.argv) < 4 else IDX
    for i in idx:
        label = cases[i][4]
        res = []
        for so in (c_so, r_so):
            p = subprocess.run([sys.executable, sys.argv[0], so, str(i), "x"],
                               capture_output=True, text=True)
            res.append(f"sig{-p.returncode}" if p.returncode < 0 else p.stdout.strip())
        same = "SAME" if res[0] == res[1] else "DIFF"
        print(f"[{i}] {label}: C={res[0]} R={res[1]} -> {same}")
