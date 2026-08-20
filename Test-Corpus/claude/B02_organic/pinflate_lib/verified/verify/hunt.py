#!/usr/bin/env python3
"""Targeted hunt for witnesses of the remaining assert() lines in lib.c.

`cp_ptr`'s `assert(!(s->bits_left & 7))` (lib.c:95) needs a stored block that is
reached after `cp_peak_bits` took its "final word" branch at a bit position that
is not byte aligned -- the only thing that can break the
`count == -consumed (mod 8)` invariant the rest of the reader maintains. So the
search is restricted to `btype == 1` followed by `btype == 0`.
"""
import ctypes, os, random, re, sys

IN_PAD = bytes(range(256)) * 32
RE = re.compile(rb"lib\.c:(\d+): (\w+): Assertion `([^']*)' failed")


def load(path):
    lib = ctypes.CDLL(path)
    lib.pinflate.restype = ctypes.c_int
    lib.pinflate.argtypes = [ctypes.c_void_p, ctypes.c_int,
                             ctypes.c_void_p, ctypes.c_int]
    return lib


def probe(lib, data, out_size, in_off):
    er, ew = os.pipe()
    pid = os.fork()
    if pid == 0:
        import signal
        signal.alarm(2)
        os.close(er)
        os.dup2(ew, 2)
        try:
            inbuf = ctypes.create_string_buffer(bytes(in_off) + data + IN_PAD)
            outbuf = ctypes.create_string_buffer(b"\xCD" * (out_size + 4096))
            ctypes.c_char_p.in_dll(lib, "cp_error_reason").value = None
            inp = ctypes.cast(inbuf, ctypes.c_void_p).value + in_off
            outp = ctypes.cast(outbuf, ctypes.c_void_p).value
            r = lib.pinflate(inp, len(data), outp, out_size)
            os._exit(100 + (1 if r else 0))
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
    if os.WIFSIGNALED(status):
        m = RE.search(b"".join(chunks))
        if m:
            return int(m.group(1))
        return -os.WTERMSIG(status)
    return 0


def main():
    lib = load(sys.argv[1])
    want = set(int(x) for x in sys.argv[2].split(","))
    n = int(sys.argv[3])
    rng = random.Random(int(sys.argv[4]) if len(sys.argv) > 4 else 1)
    hits = {}
    seen = {}
    for _ in range(n):
        ln = rng.randrange(4, 17)
        data = bytearray(rng.randrange(256) for _ in range(ln))
        # bfinal=0, btype=1 (fixed) as the first block
        data[0] = (data[0] & ~0x07) | 0x02
        in_off = rng.randrange(4)
        line = probe(lib, bytes(data), rng.choice([1, 16, 4096]), in_off)
        seen[line] = seen.get(line, 0) + 1
        if line in want and line not in hits:
            hits[line] = (bytes(data).hex(), in_off)
            print(f"HIT lib.c:{line} data={bytes(data).hex()} in_off={in_off}")
            sys.stdout.flush()
            if set(hits) >= want:
                break
    print("distribution:", dict(sorted(seen.items())))
    print("found:", sorted(hits))


main()
