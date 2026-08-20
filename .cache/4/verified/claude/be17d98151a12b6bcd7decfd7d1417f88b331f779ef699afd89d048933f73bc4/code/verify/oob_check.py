#!/usr/bin/env python3
"""The stored-block path in the C code can memcpy past the end of the input
buffer (its LEN check is `bits_left/8 <= LEN`, which is inverted). Output then
depends on whatever memory follows the input buffer.

Here the input buffer is padded with a large deterministic region so that the
"out of bounds" reads land on known bytes. C and Rust must then agree exactly.
"""
import ctypes, sys
import difftest as dt

PAD = 8192

def run_padded(lib, data, out_size, in_off):
    inbuf = ctypes.create_string_buffer(bytes(in_off) + data + bytes(range(256)) * (PAD // 256))
    outbuf = ctypes.create_string_buffer(b"\xCD" * (out_size + 70000))
    ctypes.c_char_p.in_dll(lib, "cp_error_reason").value = None
    inp = ctypes.cast(inbuf, ctypes.c_void_p).value + in_off
    outp = ctypes.cast(outbuf, ctypes.c_void_p).value
    ret = lib.pinflate(inp, len(data), outp, out_size)
    return ret, outbuf.raw, ctypes.c_char_p.in_dll(lib, "cp_error_reason").value

CASES = [
    ("01f4010bfe01020000020201000200010200000001020000020100020202020200000002000202010102020000010102010100000200000100010000020100010201000202010002010000010100020102010001000001", 4096, 2),
    ("01f4010bfe01010101010102020200020102020200", 4096, 0),
    ("01c0003fff000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f00010203040506070809", 4096, 0),
    ("01c0003fff000102030405", 64, 3),
    ("01f4010bfe01020102000200010201020002010201000201010001", 4096, 2),
    ("0105 00fa ff68656c6c6f".replace(" ", ""), 64, 0),
]

c, r = dt.load(sys.argv[1]), dt.load(sys.argv[2])
bad = 0
for hexdata, out_size, in_off in CASES:
    data = bytes.fromhex(hexdata)
    for off in range(4):
        a = run_padded(c, data, out_size, off)
        b = run_padded(r, data, out_size, off)
        ok = a == b
        if not ok:
            bad += 1
            for k, (x, y) in enumerate(zip(a[1], b[1])):
                if x != y:
                    print(f"  first diff at {k}: C={x} R={y}")
                    break
        print(f"{'OK  ' if ok else 'DIFF'} len={len(data)} out={out_size} in_off={off} "
              f"ret={a[0]}/{b[0]} err={a[2]}")
print("padded-OOB mismatches:", bad)
sys.exit(1 if bad else 0)
