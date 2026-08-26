#!/usr/bin/env python3
"""Randomized differential fuzz: C vs Rust pinflate, restartable across faults."""
import ctypes, os, random, subprocess, sys, zlib
import difftest as dt

N = int(os.environ.get("FUZZ_N", "4000"))
SEED = int(os.environ.get("FUZZ_SEED", "99"))

def build_cases():
    random.seed(SEED)
    cases = []
    corpus = []
    for lvl in (0, 1, 6, 9):
        for strat in (0, 1, 2, 3, 4):
            for pay in (b"", b"x", b"hello hello hello", bytes(range(64)) * 3,
                        bytes(random.randrange(3) for _ in range(500)),
                        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbcccc" * 7):
                co = zlib.compressobj(lvl, zlib.DEFLATED, -15, 9, strat)
                corpus.append(co.compress(pay) + co.flush())
    while len(cases) < N:
        kind = random.randrange(4)
        if kind == 0:                      # mutate a valid stream
            d = bytearray(random.choice(corpus))
            if d:
                for _ in range(random.randrange(1, 4)):
                    d[random.randrange(len(d))] ^= 1 << random.randrange(8)
            data = bytes(d)
        elif kind == 1:                    # truncate a valid stream
            d = random.choice(corpus)
            data = d[:random.randrange(0, len(d) + 1)] if d else d
        elif kind == 2:                    # pure random
            data = random.randbytes(random.randrange(0, 48))
        else:                              # random payload with a plausible header
            hdr = bytes([random.choice([0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0xEC, 0xED])])
            data = hdr + random.randbytes(random.randrange(0, 40))
        cases.append((data, random.choice([16, 64, 4096]), random.randrange(4),
                      random.randrange(2), f"fuzz{len(cases)}"))
    return cases

def worker(c_so, r_so, start, prog):
    cases = build_cases()
    c, rs = dt.load(c_so), dt.load(r_so)
    pf = open(prog, "w")
    for i in range(start, len(cases)):
        data, out_size, in_off, out_off, label = cases[i]
        pf.seek(0); pf.write(str(i) + "\n"); pf.truncate(); pf.flush()
        a = dt.run(c, data, out_size, in_off, out_off)
        b = dt.run(rs, data, out_size, in_off, out_off)
        if a != b:
            print(f"MISMATCH [{i}] {label} data={data.hex()} out={out_size} "
                  f"io={in_off}{out_off}: retC={a[0]} retR={b[0]} "
                  f"errC={a[2]!r} errR={b[2]!r} out_equal={a[1]==b[1]}")
            sys.stdout.flush()
            os._exit(4)
    pf.seek(0); pf.write(str(len(cases)) + "\n"); pf.truncate(); pf.flush()
    sys.stdout.flush()
    os._exit(0)

if __name__ == "__main__":
    if os.environ.get("FZ_WORKER"):
        worker(sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4])
    c_so, r_so = sys.argv[1], sys.argv[2]
    total = len(build_cases())
    prog = os.environ.get("TMPDIR", "/tmp") + "/fz_progress"
    env = dict(os.environ, FZ_WORKER="1")
    start, crashes, mism, fault_idx = 0, 0, 0, []
    while start < total:
        p = subprocess.run([sys.executable, sys.argv[0], c_so, r_so, str(start), prog],
                           env=env, capture_output=True, text=True)
        done = int(open(prog).read().strip())
        if p.stdout.strip():
            print(p.stdout.strip())
        if p.returncode == 0:
            start = total
        elif p.returncode == 4:
            mism += 1; start = done + 1
        else:
            crashes += 1; fault_idx.append(done); start = done + 1
    print(f"total={total} faulting_cases={crashes} mismatches={mism}")
    print("faulting_indices=" + ",".join(map(str, fault_idx)))
    sys.exit(1 if mism else 0)
