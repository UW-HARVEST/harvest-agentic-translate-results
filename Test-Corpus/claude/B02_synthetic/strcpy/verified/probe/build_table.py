#!/usr/bin/env python3
"""Turn the ptrace dumps of the C `main` frame into a Rust byte table.

For every offset the byte is classified as
  * stable            - identical in every dump   -> use that value
  * unstable          - ASLR dependent (pointer)  -> use the most common
                        non-zero value, so that the *zero pattern* (which is
                        what strlen/strcmp actually branch on) is reproduced.
"""
import collections
import glob
import os
import sys

T = os.environ["TMPDIR"]
files = sorted(glob.glob(T + "/dumps/d*.txt"))
assert files, "no dumps"

runs = []
for f in files:
    d = {}
    for line in open(f):
        o, v = line.split()
        d[int(o)] = int(v)
    runs.append(d)

n = min(len(r) for r in runs)
table = []
stats = collections.Counter()
for off in range(n):
    vals = [r[off] for r in runs]
    counter = collections.Counter(vals)
    if len(counter) == 1:
        table.append(vals[0])
        stats["stable_zero" if vals[0] == 0 else "stable_nonzero"] += 1
    else:
        # majority vote: reproduces the *zero pattern* the C code branches on
        zeros = counter[0]
        if zeros * 2 > len(vals):
            table.append(0)
            stats["unstable_mostly_zero"] += 1
        else:
            nonzero = [v for v in vals if v != 0]
            pick = collections.Counter(nonzero).most_common(1)[0][0]
            table.append(pick)
            stats["unstable_nonzero"] += 1

print("// stats:", dict(stats), file=sys.stderr)
zero_runs = sum(1 for v in table if v == 0)
print("// zero bytes:", zero_runs, "of", n, file=sys.stderr)

out = sys.stdout
out.write("//! Byte-exact snapshot of the uninitialised `main` stack frame of the C\n")
out.write("//! driver (`c_src/src/main.c`), captured from the compiled C program with a\n")
out.write("//! ptrace based dumper.  Offset 0 corresponds to `ref_buffer[0]`\n")
out.write("//! (`rbp-0x830`), offset 1024 to `input_buffer[0]` (`rbp-0x430`) and offset\n")
out.write("//! 2048 to the first local above the two arrays (`ref_len`, at `rbp-0x30`).\n")
out.write("//!\n")
out.write("//! The C code reads these bytes whenever one of its `strcmp`/`strlen` calls\n")
out.write("//! runs past the data that was actually read from stdin, so reproducing them\n")
out.write("//! is required for byte-identical output.\n")
out.write("\n")
out.write(f"pub const FRAME_JUNK: [u8; {n}] = [\n")
for base in range(0, n, 16):
    chunk = table[base:base + 16]
    out.write("    " + " ".join(f"0x{v:02x}," for v in chunk) + f" // {base}\n")
out.write("];\n")
