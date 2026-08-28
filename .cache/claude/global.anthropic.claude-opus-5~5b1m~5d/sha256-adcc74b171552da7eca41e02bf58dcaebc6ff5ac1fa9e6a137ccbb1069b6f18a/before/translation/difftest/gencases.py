#!/usr/bin/env python3
"""Generate binary test cases for difftest.c"""
import random, struct, sys, zlib

recs = []

def add(kind, name, a=0, b=0, c=0, in_off=0, in_data=b'', in_len=None,
        out_bytes=0, out_init=b''):
    if in_len is None:
        in_len = len(in_data)
    recs.append(struct.pack('<B8i', kind, a, b, c, in_off, in_len, out_bytes,
                            len(out_init), len(name)) + name.encode() +
                (in_data if in_len > 0 else b'') + out_init)

def raw_deflate(data, level=6, strategy=zlib.Z_DEFAULT_STRATEGY):
    co = zlib.compressobj(level, zlib.DEFLATED, -15, 8, strategy)
    return co.compress(data) + co.flush()

random.seed(1234)

payloads = {
    'empty': b'',
    'a': b'a',
    'text': b'the quick brown fox jumps over the lazy dog. ' * 40,
    'zeros': bytes(5000),
    'rand1k': bytes(random.randrange(256) for _ in range(1024)),
    'rand64k': bytes(random.randrange(256) for _ in range(65536)),
    'ab': (b'ab' * 3000),
    'runs': b''.join(bytes([i % 251]) * (i % 17 + 1) for i in range(900)),
    'hex': bytes.fromhex('00ff10ef') * 700,
    'binary': bytes(range(256)) * 60,
}

levels = [0, 1, 6, 9]
strategies = [zlib.Z_DEFAULT_STRATEGY, zlib.Z_FIXED, zlib.Z_HUFFMAN_ONLY, zlib.Z_RLE]
snames = {zlib.Z_DEFAULT_STRATEGY: 'def', zlib.Z_FIXED: 'fix',
          zlib.Z_HUFFMAN_ONLY: 'huf', zlib.Z_RLE: 'rle'}

streams = []  # (name, compressed, decompressed)
for pn, p in payloads.items():
    for lv in levels:
        for st in strategies:
            try:
                cd = raw_deflate(p, lv, st)
            except Exception:
                continue
            streams.append(('%s-l%d-%s' % (pn, lv, snames[st]), cd, p))

# multi-block streams via explicit flushes
for pn in ('text', 'rand1k', 'ab'):
    p = payloads[pn]
    co = zlib.compressobj(6, zlib.DEFLATED, -15)
    out = b''
    third = max(1, len(p) // 3)
    for i in range(0, len(p), third):
        out += co.compress(p[i:i + third])
        out += co.flush(zlib.Z_FULL_FLUSH)
    out += co.flush()
    streams.append(('multiblock-' + pn, out, p))

# stored (uncompressed) blocks: level 0 already gives those, plus hand made
for pn in ('text', 'rand1k'):
    p = payloads[pn]
    co = zlib.compressobj(0, zlib.DEFLATED, -15)
    streams.append(('stored-' + pn, co.compress(p) + co.flush(), p))

for name, cd, dd in streams:
    for off in (0, 1, 2, 3):
        add(0, 'inf-%s-off%d-exact' % (name, off), in_off=off, in_data=cd,
            out_bytes=len(dd))
    add(0, 'inf-%s-big' % name, in_data=cd, out_bytes=len(dd) + 4096)
    if len(dd):
        add(0, 'inf-%s-short' % name, in_data=cd, out_bytes=len(dd) - 1)
        add(0, 'inf-%s-half' % name, in_data=cd, out_bytes=len(dd) // 2)
    add(0, 'inf-%s-zero-out' % name, in_data=cd, out_bytes=0)

# truncated inputs
base = raw_deflate(payloads['text'], 6)
for cut in (0, 1, 2, 3, 4, 5, 8, 13, 30, len(base) - 1):
    if 0 <= cut <= len(base):
        add(0, 'trunc-%d' % cut, in_data=base[:cut], in_len=cut,
            out_bytes=len(payloads['text']))

# bad LEN/NLEN in stored block, unknown block type, etc.
add(0, 'btype3', in_data=bytes([0b111]) + bytes(16), out_bytes=100)
add(0, 'stored-bad-nlen', in_data=bytes([0x00, 0x05, 0x00, 0x00, 0x00]) + b'hello' + bytes(4),
    out_bytes=100)
add(0, 'stored-good', in_data=bytes([0x01, 0x05, 0x00, 0xFA, 0xFF]) + b'hello' + bytes(4),
    out_bytes=100)
add(0, 'stored-good-small-out', in_data=bytes([0x01, 0x05, 0x00, 0xFA, 0xFF]) + b'hello' + bytes(4),
    out_bytes=2)
add(0, 'empty-in', in_data=b'', in_len=0, out_bytes=64)

# fuzzed / bit flipped valid streams (may legitimately crash both libs)
for i in range(120):
    cd = bytearray(raw_deflate(payloads[random.choice(list(payloads))], random.choice(levels)))
    if not cd:
        continue
    for _ in range(random.randrange(1, 4)):
        pos = random.randrange(len(cd))
        cd[pos] ^= 1 << random.randrange(8)
    add(0, 'flip%d' % i, in_data=bytes(cd), out_bytes=random.choice([64, 1024, 65536]))

for i in range(80):
    n = random.randrange(1, 200)
    data = bytes(random.randrange(256) for _ in range(n))
    add(0, 'garbage%d' % i, in_data=data, out_bytes=random.choice([0, 16, 4096]))

# ---------------- unfilter cases ----------------
def unf_buf(w, h, bpp, filters=None, seed=0):
    rnd = random.Random(seed)
    rows = []
    for y in range(max(h, 0)):
        f = filters[y % len(filters)] if filters else rnd.randrange(5)
        rows.append(bytes([f]) + bytes(rnd.randrange(256) for _ in range(max(w * bpp, 0))))
    return b''.join(rows)

ucount = 0
for (w, h, bpp) in [(1, 1, 1), (1, 1, 4), (8, 8, 3), (8, 8, 4), (17, 5, 2),
                    (32, 32, 4), (100, 3, 1), (7, 11, 8), (64, 64, 3), (5, 1, 4),
                    (3, 9, 1), (16, 2, 4)]:
    for filters in ([0], [1], [2], [3], [4], None, [0, 1, 2, 3, 4], [4, 3, 2, 1, 0]):
        buf = unf_buf(w, h, bpp, filters, seed=ucount)
        add(1, 'unf-%dx%d-bpp%d-%s' % (w, h, bpp, 'rnd' if filters is None else ''.join(map(str, filters))),
            a=w, b=h, c=bpp, out_init=buf)
        ucount += 1

# invalid filter bytes and degenerate dimensions
for (w, h, bpp, filters) in [(8, 4, 3, [5]), (8, 4, 3, [0, 9]), (8, 4, 3, [255]),
                             (8, 0, 3, [0]), (0, 4, 3, [0]), (8, 4, 0, [1]),
                             (8, -3, 3, [0]), (-8, 4, 3, [0]), (8, 4, -1, [2]),
                             (2, 3, 100, [1]), (1, 300, 1, None)]:
    buf = unf_buf(max(w, 0), max(h, 0), max(bpp, 0), filters, seed=ucount)
    if not buf:
        buf = bytes(random.randrange(256) for _ in range(64))
    add(1, 'unf-edge-%d-%d-%d-%s' % (w, h, bpp, 'rnd' if filters is None else ''.join(map(str, filters))),
        a=w, b=h, c=bpp, out_init=buf)
    ucount += 1

for i in range(60):
    w = random.randrange(1, 40); h = random.randrange(1, 20); bpp = random.randrange(1, 9)
    buf = unf_buf(w, h, bpp, None, seed=1000 + i)
    add(1, 'unf-rand%d' % i, a=w, b=h, c=bpp, out_init=buf)

with open(sys.argv[1], 'wb') as f:
    f.write(b''.join(recs))
print('wrote %d cases' % len(recs))
