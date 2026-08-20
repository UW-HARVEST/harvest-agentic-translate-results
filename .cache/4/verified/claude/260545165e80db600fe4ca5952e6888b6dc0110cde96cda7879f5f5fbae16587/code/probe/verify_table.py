#!/usr/bin/env python3
"""Cross-check the ptrace derived junk table against the *observable* behaviour
of the C driver: for every offset k the probe below reveals whether
input_buffer[k] / ref_buffer[k] is zero."""
import re
import subprocess
import sys

DRIVER = "c_src/build/driver"
TABLE = sys.argv[1]
MAXK = int(sys.argv[2]) if len(sys.argv) > 2 else 1023

src = open(TABLE).read()
nums = re.findall(r"0x([0-9a-f]{2}),", src)
table = [int(x, 16) for x in nums]
print("table len", len(table))


def run(tokens):
    p = subprocess.run([DRIVER], input=" ".join(str(t) for t in tokens),
                       capture_output=True, text=True)
    return p.returncode, p.stdout.strip()


bad = 0
checked = 0
for k in range(0, MAXK + 1):
    # input_buffer[k] == 0  <=>  result 6 (k>0)
    rc, so = run([4, 0, k] + [65] * k + [k + 1] + [97] * k + [0])
    if rc == 0 and k > 0:
        obs_zero = (so == "6")
        exp_zero = (table[1024 + k] == 0)
        checked += 1
        if obs_zero != exp_zero:
            bad += 1
            if bad < 20:
                print(f"input_buffer[{k}]: observed zero={obs_zero} table={table[1024+k]:#02x} (res={so})")
    # ref_buffer[k] == 0
    rc, so = run([4, 0, k + 1] + [65] * k + [0] + [k] + [97] * k)
    if rc == 0 and k > 0:
        obs_zero = (so == "6")
        exp_zero = (table[k] == 0)
        checked += 1
        if obs_zero != exp_zero:
            bad += 1
            if bad < 20:
                print(f"ref_buffer[{k}]: observed zero={obs_zero} table={table[k]:#02x} (res={so})")
print(f"checked={checked} mismatches={bad}")
