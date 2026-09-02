#!/usr/bin/env python3
"""Randomized differential fuzzer: generate command scripts, run both binaries,
compare stdout/stderr/exit status byte for byte."""
import random, subprocess, sys, os

C = "c_src/build/driver"
R = "translation/target/release/driver"

CMDS = ["adduser","login","logout","whoami","listusers","users","createfile","touch",
        "readfile","cat","writefile","write","deletefile","rm","listfiles","ls",
        "set","get","unset","listvars","vars","compare","cmp","compareN","cmpn",
        "startswith","match","debug","verbose","status","help","?",
        # near-miss prefixes that hit the strncmp "did you mean" branches
        "add","addx","log","logx","list","listx","create","createx","read","readx",
        "writex","delete","deletex","bogus","", " ", "\t"]

NAMES = ["a","b","alice","bob","u1","f1","x","y","on","off","0","1","5","9","-1",
         "abc","abd","ABC","hello","he",
         "A"*31, "A"*32, "A"*33, "A"*40, "A"*63, "A"*64,
         "P"*36, "P"*39, "P"*40, "P"*63,
         "99999999999999999999", "2147483648", "-2147483649", "abc123", "0x10"]

def gen(rng):
    lines = []
    for _ in range(rng.randint(1, 40)):
        c = rng.choice(CMDS)
        n = rng.randint(0, 3)
        parts = [c] + [rng.choice(NAMES) for _ in range(n)]
        sep = rng.choice([" ", "  ", "\t", " \t "])
        lines.append(sep.join(p for p in parts))
    return ("\n".join(lines) + rng.choice(["\n", ""])).encode()

def run(path, data):
    p = subprocess.run([path], input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return p.stdout, p.stderr, p.returncode

def main():
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 500
    seed0 = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    fails = 0
    for i in range(n):
        rng = random.Random(seed0 + i)
        data = gen(rng)
        co, ce, cs = run(C, data)
        ro, re_, rs = run(R, data)
        if (co, ce, cs) != (ro, re_, rs):
            fails += 1
            print(f"=== MISMATCH seed={seed0+i} ===")
            print("input:", data[:400])
            if co != ro:
                print(f"  stdout differs: C={len(co)}B RUST={len(ro)}B")
                for k in range(min(len(co), len(ro))):
                    if co[k] != ro[k]:
                        print(f"  first diff at byte {k}: C={co[max(0,k-60):k+60]!r}")
                        print(f"                          R={ro[max(0,k-60):k+60]!r}")
                        break
            if ce != re_:
                print(f"  stderr differs: C={ce[:200]!r} RUST={re_[:200]!r}")
            if cs != rs:
                print(f"  exit differs: C={cs} RUST={rs}")
            if fails >= 5:
                print("stopping after 5 mismatches")
                break
    print(f"{n - fails}/{n} matched" if fails else f"ALL {n} MATCHED")
    return 1 if fails else 0

sys.exit(main())
