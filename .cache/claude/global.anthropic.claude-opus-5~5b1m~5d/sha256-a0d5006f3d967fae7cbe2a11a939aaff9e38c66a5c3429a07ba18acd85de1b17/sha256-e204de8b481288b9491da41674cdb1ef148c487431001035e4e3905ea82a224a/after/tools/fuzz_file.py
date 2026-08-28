#!/usr/bin/env python3
"""Random differential fuzzing of the scene-file (save/load) paths."""
import random
import sys

from cmp import C, R, norm, run

LINES = [b"", b"S", b"Scene name", b"N" * 63, b"N" * 70, b"0", b"1", b"2", b"3",
         b"9", b"10", b"-1", b"50", b"51", b"55", b"x", b"3junk", b"  2  ",
         b"+2", b"2 1 3", b"99999999999", b"\xff\xfe", b"0\r", b"\t7"]


def gen(rng):
    n = rng.randint(0, 8)
    body = b"\n".join(rng.choice(LINES) for _ in range(n))
    if rng.random() < 0.7:
        body += b"\n"
    return body


def main():
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    iters = int(sys.argv[2]) if len(sys.argv) > 2 else 200
    rng = random.Random(seed)
    stdin = b"8\nf.txt\n6\n5\n0\n7\n0\ng.txt\n8\ng.txt\n6\n12\n"
    fails = 0
    for _ in range(iters):
        content = gen(rng)
        files = {"f.txt": content}
        c = run(C, stdin, files, timeout=5)
        r = run(R, stdin, files, timeout=5)
        if c[0] != r[0] or norm(c[1]) != norm(r[1]) or norm(c[2]) != norm(r[2]) \
                or c[3] != r[3]:
            fails += 1
            print("=== MISMATCH file=%r" % content)
            print("  status C=%r R=%r" % (c[0], r[0]))
            a, b = norm(c[1]), norm(r[1])
            if a != b:
                k = next((j for j in range(min(len(a), len(b))) if a[j] != b[j]),
                         min(len(a), len(b)))
                print("  stdout at %d:\n   C=%r\n   R=%r" % (k, a[k:k + 150], b[k:k + 150]))
            if norm(c[2]) != norm(r[2]):
                print("  stderr C=%r R=%r" % (c[2][:150], r[2][:150]))
            if c[3] != r[3]:
                print("  files C=%r\n        R=%r" % (c[3], r[3]))
    print("done: %d/%d mismatches" % (fails, iters))


if __name__ == "__main__":
    main()
