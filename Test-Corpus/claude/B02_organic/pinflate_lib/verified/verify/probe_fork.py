#!/usr/bin/env python3
"""Fast fork-isolated differential probe (C aborts on assert failures, so each
case must run in its own process).  Both libraries are loaded in the parent;
each case is executed in a forked child which reports (ret, err, out-digest)
over a pipe.  A child that dies by signal reports the signal instead.
"""
import ctypes, hashlib, os, random, struct, sys, zlib

IN_PAD = bytes(range(256)) * 32
OUT_PAD = 4096


def load(path):
    lib = ctypes.CDLL(path)
    lib.pinflate.restype = ctypes.c_int
    lib.pinflate.argtypes = [ctypes.c_void_p, ctypes.c_int,
                             ctypes.c_void_p, ctypes.c_int]
    return lib


def run_isolated(lib, data, out_size, in_off, out_off):
    r, w = os.pipe()
    pid = os.fork()
    if pid == 0:
        os.close(r)
        # pinflate can legitimately loop forever (length-code 286/287 have
        # cp_len_base == 0, so a match of length 0 makes no progress); bound the
        # child so C and Rust are compared as "both hang".
        import signal
        signal.alarm(3)
        try:
            inbuf = ctypes.create_string_buffer(bytes(in_off) + data + IN_PAD)
            outbuf = ctypes.create_string_buffer(
                b"\xCD" * (out_off + out_size + OUT_PAD))
            ctypes.c_char_p.in_dll(lib, "cp_error_reason").value = None
            inp = ctypes.cast(inbuf, ctypes.c_void_p).value + in_off
            outp = ctypes.cast(outbuf, ctypes.c_void_p).value + out_off
            ret = lib.pinflate(inp, len(data), outp, out_size)
            err = ctypes.c_char_p.in_dll(lib, "cp_error_reason").value
            dig = hashlib.sha256(outbuf.raw).hexdigest()[:16]
            payload = repr((ret, err, dig)).encode()
            os.write(w, payload)
        except BaseException as e:  # noqa
            os.write(w, b"pyexc:" + repr(e).encode())
        os.close(w)
        os._exit(0)
    os.close(w)
    chunks = []
    while True:
        c = os.read(r, 65536)
        if not c:
            break
        chunks.append(c)
    os.close(r)
    _, status = os.waitpid(pid, 0)
    if os.WIFSIGNALED(status):
        return "sig%d" % os.WTERMSIG(status)
    return b"".join(chunks).decode()


def build_cases(seed, n):
    random.seed(seed)
    corpus = []
    for lvl in (0, 1, 6, 9):
        for strat in (0, 1, 2, 3, 4):
            for pay in (b"", b"x", b"hello hello hello", bytes(range(64)) * 3,
                        bytes(random.randrange(3) for _ in range(500)),
                        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbcccc" * 7):
                co = zlib.compressobj(lvl, zlib.DEFLATED, -15, 9, strat)
                corpus.append(co.compress(pay) + co.flush())
    cases = []
    while len(cases) < n:
        kind = random.randrange(5)
        if kind == 0:
            d = bytearray(random.choice(corpus))
            if d:
                for _ in range(random.randrange(1, 4)):
                    d[random.randrange(len(d))] ^= 1 << random.randrange(8)
            data = bytes(d)
        elif kind == 1:
            d = random.choice(corpus)
            data = d[:random.randrange(0, len(d) + 1)] if d else d
        elif kind == 2:
            data = random.randbytes(random.randrange(0, 48))
        elif kind == 3:
            hdr = bytes([random.choice([0x00, 0x01, 0x02, 0x03, 0x04, 0x05,
                                        0xEC, 0xED, 0x1D, 0x25])])
            data = hdr + random.randbytes(random.randrange(0, 40))
        else:
            # dynamic-block headers with maximal nlit/ndst -> exercises the
            # `lens[288+32]` overrun in cp_dynamic
            bits = [1, 0, 1]                    # bfinal=1, btype=2
            bits += [1] * 5                     # nlit = 288
            bits += [1] * 5                     # ndst = 32
            for b in range(4):                  # nlen
                bits.append((15 >> b) & 1)
            bits += [random.randrange(2) for _ in range(random.randrange(0, 300))]
            by = bytearray((len(bits) + 7) // 8)
            for i, b in enumerate(bits):
                if b:
                    by[i >> 3] |= 1 << (i & 7)
            data = bytes(by)
        cases.append((data, random.choice([16, 64, 4096]),
                      random.randrange(4), random.randrange(2)))
    return cases


def main():
    c_so, r_so, seed, n = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
    c, r = load(c_so), load(r_so)
    cases = build_cases(seed, n)
    same = diff = 0
    shown = 0
    for i, case in enumerate(cases):
        a = run_isolated(c, *case)
        b = run_isolated(r, *case)
        if a == b:
            same += 1
        else:
            diff += 1
            if shown < 20:
                shown += 1
                print(f"[{i}] data={case[0].hex()} out={case[1]} io={case[2]} "
                      f"oo={case[3]}\n     C={a}\n     R={b}")
    print(f"seed={seed} n={n} same={same} diff={diff}")
    sys.exit(1 if diff else 0)


main()
