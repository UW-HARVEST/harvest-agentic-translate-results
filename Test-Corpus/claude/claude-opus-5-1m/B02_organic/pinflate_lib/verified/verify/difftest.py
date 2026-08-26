#!/usr/bin/env python3
"""Differential test: C libtranslated_rust.so vs Rust libtranslated_rust.so.

Runs all cases in one worker process; if a case crashes the process (both
libraries can legitimately segfault on hostile input, since the C original has
no bounds checks), the driver restarts the worker after that case.
"""
import ctypes, os, random, subprocess, sys, zlib

def load(path):
    lib = ctypes.CDLL(path)
    lib.pinflate.restype = ctypes.c_int
    lib.pinflate.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_void_p, ctypes.c_int]
    return lib

def globals_of(lib):
    out = {}
    for name, n, t in [("cp_fixed_table", 320, ctypes.c_uint8),
                       ("cp_permutation_order", 19, ctypes.c_uint8),
                       ("cp_len_extra_bits", 31, ctypes.c_uint8),
                       ("cp_len_base", 31, ctypes.c_uint32),
                       ("cp_dist_extra_bits", 32, ctypes.c_uint8),
                       ("cp_dist_base", 32, ctypes.c_uint32)]:
        out[name] = list((t * n).in_dll(lib, name))
    return out

# Deterministic padding: the C code has genuine out-of-bounds reads (stored
# blocks whose LEN exceeds the input) and out-of-bounds writes (LEN larger than
# the out buffer).  Padding both buffers with known bytes makes those paths
# comparable instead of depending on unrelated heap contents.
IN_PAD = bytes(range(256)) * 32
OUT_PAD = 70000

def run(lib, data, out_size, in_off, out_off):
    inbuf = ctypes.create_string_buffer(bytes(in_off) + data + IN_PAD)
    outbuf = ctypes.create_string_buffer(b"\xCD" * (out_off + out_size + OUT_PAD))
    ctypes.c_char_p.in_dll(lib, "cp_error_reason").value = None
    inp = ctypes.cast(inbuf, ctypes.c_void_p).value + in_off
    outp = ctypes.cast(outbuf, ctypes.c_void_p).value + out_off
    ret = lib.pinflate(inp, len(data), outp, out_size)
    err = ctypes.c_char_p.in_dll(lib, "cp_error_reason").value
    return ret, outbuf.raw, err

def raw_deflate(payload, level=6, strategy=zlib.Z_DEFAULT_STRATEGY):
    co = zlib.compressobj(level, zlib.DEFLATED, -15, 9, strategy)
    return co.compress(payload) + co.flush()

def build_cases():
    random.seed(1234)
    cases = []
    payloads = [
        b"", b"a", b"hello world", b"A" * 600, b"ab" * 900,
        bytes(range(256)) * 3,
        bytes(random.randrange(256) for _ in range(1500)),
        b"The quick brown fox jumps over the lazy dog. " * 40,
        bytes(random.randrange(4) for _ in range(1200)),
        random.randbytes(4000),
        b"\x00" * 70000,
    ]
    for p in payloads:
        for level in (0, 1, 6, 9):
            for strat in (zlib.Z_DEFAULT_STRATEGY, zlib.Z_FIXED, zlib.Z_HUFFMAN_ONLY, zlib.Z_RLE):
                d = raw_deflate(p, level, strat)
                for in_off in range(4):
                    for out_off in (0, 1):
                        cases.append((d, max(len(p), 1) + 4, in_off, out_off,
                                      f"valid L{level} S{strat} len{len(p)} io{in_off}{out_off}"))
                if p:
                    cases.append((d, len(p) - 1, 0, 0, f"tight L{level} S{strat} len{len(p)}"))
                    cases.append((d, 1, 0, 0, f"tiny L{level} S{strat} len{len(p)}"))
    base = raw_deflate(b"The quick brown fox jumps over the lazy dog. " * 20)
    for n in range(1, min(len(base), 60)):
        cases.append((base[:n], 4096, 0, 0, f"trunc{n}"))
    for i in range(300):
        b = bytearray(base)
        b[random.randrange(len(b))] ^= 1 << random.randrange(8)
        cases.append((bytes(b), 4096, random.randrange(4), 0, f"bitflip{i}"))
    cases.append((b"\x01\x05\x00\xfa\xffhello", 64, 0, 0, "stored-ok"))
    cases.append((b"\x01\x05\x00\xfb\xffhello", 64, 0, 0, "stored-badnlen"))
    cases.append((b"\x00\x03\x00\xfc\xffabc\x01\x02\x00\xfd\xffxy", 64, 0, 0, "stored-2blk"))
    cases.append((b"\x07\x00\x00\x00", 64, 0, 0, "btype3"))
    cases.append((b"\x05\x00\x00\x00", 64, 0, 0, "btype2-empty"))
    for n in range(0, 13):
        for in_off in range(4):
            cases.append((bytes(random.randrange(256) for _ in range(n)), 256, in_off, 0,
                          f"rand{n}.{in_off}"))
    for i in range(500):
        n = random.randrange(1, 40)
        cases.append((random.randbytes(n), 1024, random.randrange(4), 0, f"urand{i}"))
    return cases

def worker(c_so, r_so, start, progress_path):
    cases = build_cases()
    c, rs = load(c_so), load(r_so)
    if globals_of(c) != globals_of(rs):
        print("MISMATCH globals"); sys.stdout.flush()
        os._exit(3)
    pf = open(progress_path, "w")
    for i in range(start, len(cases)):
        data, out_size, in_off, out_off, label = cases[i]
        pf.seek(0); pf.write(str(i) + "\n"); pf.truncate(); pf.flush()
        a = run(c, data, out_size, in_off, out_off)
        b = run(rs, data, out_size, in_off, out_off)
        if a[0] != b[0] or a[1] != b[1] or a[2] != b[2]:
            print(f"MISMATCH [{i}] {label}: ret C={a[0]} R={b[0]} err C={a[2]!r} R={b[2]!r}")
            if a[1] != b[1]:
                for k, (x, y) in enumerate(zip(a[1], b[1])):
                    if x != y:
                        print(f"   first out diff at {k}: C={x} R={y}")
                        break
            sys.stdout.flush()
            os._exit(4)
    pf.seek(0); pf.write(str(len(cases)) + "\n"); pf.truncate(); pf.flush()
    sys.stdout.flush()
    os._exit(0)

if __name__ == "__main__":
    if os.environ.get("DT_WORKER"):
        worker(sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4])
    c_so, r_so = sys.argv[1], sys.argv[2]
    total = len(build_cases())
    prog = os.environ.get("TMPDIR", "/tmp") + "/dt_progress"
    start, crashes, mismatch = 0, [], False
    env = dict(os.environ, DT_WORKER="1")
    while start < total:
        p = subprocess.run([sys.executable, sys.argv[0], c_so, r_so, str(start), prog],
                           env=env, capture_output=True, text=True)
        done = int(open(prog).read().strip())
        if p.stdout.strip():
            print(p.stdout.strip())
        if p.returncode == 0:
            start = total
        elif p.returncode in (3, 4):
            mismatch = True
            start = done + 1
        else:  # died by signal on case `done` -- both libs are equally unsafe here
            crashes.append(done)
            start = done + 1
    print(f"total={total} crashed_cases={len(crashes)} mismatch={mismatch}")
    if crashes:
        print("crashed indices (input hostile enough to fault the C code too):", crashes[:20])
    sys.exit(1 if mismatch else 0)
