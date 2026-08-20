"""Valid-stream-only comparison (used to check the assert-enabled C build)."""
import sys, zlib, random, ctypes
import difftest as dt
c, r = dt.load(sys.argv[1]), dt.load(sys.argv[2])
random.seed(5)
bad = ok = 0
payloads = [b"", b"a", b"hi", b"hello world", b"A"*600, b"ab"*900, bytes(range(256))*3,
            bytes(random.randrange(256) for _ in range(1500)),
            b"The quick brown fox jumps over the lazy dog. "*40,
            bytes(random.randrange(4) for _ in range(1200)), random.randbytes(4000),
            b"\x00"*70000, random.randbytes(65536)]
for p in payloads:
    for lvl in (0,1,2,6,9):
        for strat in (0,1,2,3,4):
            co = zlib.compressobj(lvl, zlib.DEFLATED, -15, 9, strat)
            d = co.compress(p) + co.flush()
            for io in range(4):
                for oo in (0,1,3):
                    a = dt.run(c, d, max(len(p),1), io, oo)
                    b = dt.run(r, d, max(len(p),1), io, oo)
                    if a != b:
                        bad += 1
                        print(f"MISMATCH len={len(p)} lvl={lvl} strat={strat} io={io}{oo} "
                              f"ret={a[0]}/{b[0]} err={a[2]!r}/{b[2]!r} out_eq={a[1]==b[1]}")
                    else:
                        ok += 1
print(f"valid-stream cases ok={ok} bad={bad}")
sys.exit(1 if bad else 0)
