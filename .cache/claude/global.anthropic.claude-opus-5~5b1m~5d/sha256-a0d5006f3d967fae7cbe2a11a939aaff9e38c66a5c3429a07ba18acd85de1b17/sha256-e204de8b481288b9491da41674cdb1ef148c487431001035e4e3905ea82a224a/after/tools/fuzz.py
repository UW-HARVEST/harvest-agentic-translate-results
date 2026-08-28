#!/usr/bin/env python3
"""Random differential fuzzing of the C and Rust drivers."""
import random
import sys

from cmp import C, R, norm, run

TOKENS = [b"1", b"2", b"3", b"4", b"5", b"6", b"7", b"8", b"9", b"10", b"11",
          b"0", b"-1", b"12abc", b"x", b"", b"  7 ", b"99999999999999999999",
          b"4294967298", b"+2", b"0x3", b"A name", b"f.txt", b"nope.txt",
          b".", b"2 3", b"\t5", b"-2147483648", b"50", b"51", b"N" * 70]


def gen(rng):
    n = rng.randint(1, 25)
    parts = [rng.choice(TOKENS) for _ in range(n)]
    data = b"\n".join(parts)
    if rng.random() < 0.8:
        data += b"\n12\n"   # keep most runs terminating
    return data


def main():
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    iters = int(sys.argv[2]) if len(sys.argv) > 2 else 200
    rng = random.Random(seed)
    fails = 0
    for i in range(iters):
        data = gen(rng)
        files = {"f.txt": b"Saved\n2\n1\n3\n"}
        c = run(C, data, files, timeout=5)
        r = run(R, data, files, timeout=5)
        co, ro = norm(c[1]), norm(r[1])
        if c[0] == "TIMEOUT" and r[0] == "TIMEOUT":
            # Killed mid-run: only the 4096-byte-aligned flushed prefix
            # survives.  Because C's and Rust's heap addresses differ in
            # textual width, the two prefixes stop at different logical
            # positions; require the shorter to be a prefix of the longer.
            k = max(0, min(len(co), len(ro)) - 16)
            co, ro = co[:k], ro[:k]
        if c[0] != r[0] or co != ro or norm(c[2]) != norm(r[2]) \
                or c[3] != r[3]:
            fails += 1
            print("=== MISMATCH on %r" % data)
            print("  status C=%r R=%r" % (c[0], r[0]))
            if norm(c[1]) != norm(r[1]):
                a, b = norm(c[1]), norm(r[1])
                k = next((j for j in range(min(len(a), len(b))) if a[j] != b[j]),
                         min(len(a), len(b)))
                print("  stdout diff at %d:\n   C=%r\n   R=%r" % (k, a[k:k + 120], b[k:k + 120]))
            if norm(c[2]) != norm(r[2]):
                print("  stderr C=%r R=%r" % (c[2][:200], r[2][:200]))
            if c[3] != r[3]:
                print("  files C=%r R=%r" % (c[3], r[3]))
    print("done: %d/%d mismatches" % (fails, iters))


if __name__ == "__main__":
    main()
