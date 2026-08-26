#!/usr/bin/env python3
"""Find a witness input for every assert() line in c_src/src/lib.c.

Runs hostile inputs against the assert-enabled C .so in a forked child with
stderr captured, and records the first input that trips each distinct
`lib.c:<line>` assertion. Those witnesses become the Phase C error-path rows.
"""
import ctypes, os, random, re, sys, zlib

IN_PAD = bytes(range(256)) * 32
OUT_PAD = 4096
RE = re.compile(rb"lib\.c:(\d+): (\w+): Assertion `([^']*)' failed")


def load(path):
    lib = ctypes.CDLL(path)
    lib.pinflate.restype = ctypes.c_int
    lib.pinflate.argtypes = [ctypes.c_void_p, ctypes.c_int,
                             ctypes.c_void_p, ctypes.c_int]
    return lib


def probe(lib, data, out_size, in_off, out_off):
    """-> ('ret', n) | ('assert', line, func, expr) | ('sig', n)"""
    er, ew = os.pipe()
    pid = os.fork()
    if pid == 0:
        import signal
        signal.alarm(3)
        os.close(er)
        os.dup2(ew, 2)
        try:
            inbuf = ctypes.create_string_buffer(bytes(in_off) + data + IN_PAD)
            outbuf = ctypes.create_string_buffer(
                b"\xCD" * (out_off + out_size + OUT_PAD))
            ctypes.c_char_p.in_dll(lib, "cp_error_reason").value = None
            inp = ctypes.cast(inbuf, ctypes.c_void_p).value + in_off
            outp = ctypes.cast(outbuf, ctypes.c_void_p).value + out_off
            ret = lib.pinflate(inp, len(data), outp, out_size)
            os._exit(100 + (1 if ret else 0))
        except BaseException:
            os._exit(120)
    os.close(ew)
    chunks = []
    while True:
        c = os.read(er, 65536)
        if not c:
            break
        chunks.append(c)
    os.close(er)
    _, status = os.waitpid(pid, 0)
    err = b"".join(chunks)
    if os.WIFSIGNALED(status):
        m = RE.search(err)
        if m:
            return ("assert", int(m.group(1)), m.group(2).decode(),
                    m.group(3).decode())
        return ("sig", os.WTERMSIG(status))
    return ("ret", os.WEXITSTATUS(status) - 100)


def corpus(rng):
    out = []
    for lvl in (0, 1, 6, 9):
        for strat in (0, 1, 2, 3, 4):
            for pay in (b"", b"x", b"hello hello hello", bytes(range(64)) * 3,
                        bytes(rng.randrange(3) for _ in range(400))):
                co = zlib.compressobj(lvl, zlib.DEFLATED, -15, 9, strat)
                out.append(co.compress(pay) + co.flush())
    return out


def gen(rng, corp):
    kind = rng.randrange(6)
    if kind == 0:
        d = bytearray(rng.choice(corp))
        if d:
            for _ in range(rng.randrange(1, 5)):
                d[rng.randrange(len(d))] ^= 1 << rng.randrange(8)
        data = bytes(d)
    elif kind == 1:
        d = rng.choice(corp)
        data = d[:rng.randrange(0, len(d) + 1)] if d else d
    elif kind == 2:
        data = bytes(rng.randrange(256) for _ in range(rng.randrange(0, 48)))
    elif kind == 3:
        hdr = bytes([rng.choice([0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0xEC, 0xED])])
        data = hdr + bytes(rng.randrange(256) for _ in range(rng.randrange(0, 40)))
    elif kind == 4:
        # dynamic header with maximal nlit/ndst -> lens[] overrun
        bits = [1, 0, 1] + [1] * 10 + [1, 1, 1, 1]
        bits += [rng.randrange(2) for _ in range(rng.randrange(0, 320))]
        by = bytearray((len(bits) + 7) // 8)
        for i, b in enumerate(bits):
            if b:
                by[i >> 3] |= 1 << (i & 7)
        data = bytes(by)
    else:
        # a stored block preceded by a fixed block -> exercises cp_ptr
        pre = rng.randrange(0, 40)
        bits = [rng.randrange(2) for _ in range(pre)]
        by = bytearray([0x02 | 0x00])
        by += bytes(rng.randrange(256) for _ in range(rng.randrange(0, 24)))
        data = bytes(by)
        _ = bits
    return (data, rng.choice([0, 1, 16, 64, 4096]), rng.randrange(4),
            rng.randrange(2))


def main():
    lib = load(sys.argv[1])
    n = int(sys.argv[2]) if len(sys.argv) > 2 else 6000
    rng = random.Random(int(sys.argv[3]) if len(sys.argv) > 3 else 4242)
    corp = corpus(rng)
    found = {}
    other = {}
    for _ in range(n):
        case = gen(rng, corp)
        r = probe(lib, *case)
        if r[0] == "assert":
            key = (r[1], r[2], r[3])
            if key not in found:
                found[key] = case
        elif r[0] == "sig":
            other.setdefault(r[1], case)
    print("=== assert witnesses ===")
    for (line, func, expr), case in sorted(found.items()):
        data, osz, io_, oo = case
        print(f"lib.c:{line} {func}: `{expr}`")
        print(f"    data={data.hex() or '-'} out_size={osz} in_off={io_} out_off={oo}")
    print("=== non-assert fatal signals ===")
    for sig, case in sorted(other.items()):
        data, osz, io_, oo = case
        print(f"signal {sig}: data={case[0].hex() or '-'} out_size={osz} "
              f"in_off={io_} out_off={oo}")


main()
