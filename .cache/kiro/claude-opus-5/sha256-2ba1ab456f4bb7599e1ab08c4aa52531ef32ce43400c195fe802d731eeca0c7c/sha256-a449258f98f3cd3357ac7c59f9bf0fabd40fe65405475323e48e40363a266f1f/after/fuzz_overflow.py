#!/usr/bin/env python3
"""Differential fuzzer biased toward the states where the C's buffer overflows
become observable: nearly-full arrays and long-named (overflowed) owners."""
import random, subprocess, sys

C = "c_src/build/driver"
R = "translation/target/release/driver"

def run(path, data):
    p = subprocess.run([path], input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return p.stdout, p.stderr, p.returncode

LONG = ["A"*33, "A"*36, "A"*38, "A"*40, "A"*47, "A"*55, "A"*63,
        "P"*33, "P"*36, "P"*39, "P"*40, "P"*48, "P"*63]

def gen(rng):
    lines = []
    # Prologue: create a user whose stored name overflows into its password,
    # then log in under the mangled name so file owners are over-long.
    style = rng.randint(0, 3)
    if style == 0:
        uname, pw = rng.choice(LONG), rng.choice(["secret", "pw", "P"*rng.randint(1, 40)])
        lines.append(f"adduser {uname} {pw} {rng.choice([1,5,9])}")
        mangled = uname[:32] + pw
        lines.append(f"login {mangled} {pw}")
    elif style == 1:
        lines.append(f"adduser u p {rng.choice([1,5,9])}")
        lines.append("login u p")
    elif style == 2:
        # fill users up to the 10th, then a long password on the last
        for i in range(rng.randint(7, 9)):
            lines.append(f"adduser u{i} p{i} {i}")
        lines.append(f"adduser last {rng.choice(LONG)} 5")
    else:
        for i in range(rng.randint(1, 3)):
            lines.append(f"adduser u{i} {rng.choice(LONG)} {i}")
        lines.append("login u0 p")

    # Fill files / variables to near or past capacity
    for i in range(rng.randint(0, 22)):
        lines.append(f"createfile f{i} c{i}")
    for i in range(rng.randint(0, 22)):
        lines.append(f"set v{i} {rng.choice(['x', 'A'*rng.randint(1,63)])}")

    # Then random observation / mutation commands
    obs = ["listfiles","listusers","listvars","status","whoami","logout",
           "readfile f0","readfile f19","writefile f0 z","deletefile f0",
           "deletefile f19","get v0","unset v0","help","ls","users","vars"]
    for _ in range(rng.randint(1, 12)):
        lines.append(rng.choice(obs))
    return ("\n".join(lines) + "\n").encode()

def main():
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 300
    seed0 = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    fails = 0
    crashes = 0
    for i in range(n):
        rng = random.Random(seed0 + i)
        data = gen(rng)
        co, ce, cs = run(C, data)
        ro, re_, rs = run(R, data)
        if cs != 0:
            crashes += 1
        if (co, ce, cs) != (ro, re_, rs):
            fails += 1
            print(f"=== MISMATCH seed={seed0+i} ===")
            print("input:", data[:600])
            if cs != rs:
                print(f"  exit: C={cs} RUST={rs}")
            if co != ro:
                print(f"  stdout: C={len(co)}B RUST={len(ro)}B")
                for k in range(min(len(co), len(ro))):
                    if co[k] != ro[k]:
                        print(f"  first diff byte {k}:\n   C={co[max(0,k-80):k+80]!r}\n   R={ro[max(0,k-80):k+80]!r}")
                        break
            if ce != re_:
                print(f"  stderr: C={ce[:200]!r} RUST={re_[:200]!r}")
            if fails >= 5:
                break
    print(f"{n-fails}/{n} matched; {crashes} of {n} cases crashed the C (signal paths exercised)")
    return 1 if fails else 0

sys.exit(main())
