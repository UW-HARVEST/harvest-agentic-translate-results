#!/usr/bin/env python3
"""Generate test fixtures for the C-vs-Rust PNG loader comparison tests.

Outputs into translation/tests/fixtures/:
  deflate/<name>.bin      raw DEFLATE streams
  deflate/manifest.txt    "<name> <uncompressed_len>"
  png/<name>.png          PNG files
  png/manifest.txt        "<name>"
"""
import os
import struct
import zlib
import random

ROOT = os.path.dirname(os.path.abspath(__file__))
FIX = os.path.join(ROOT, "translation", "tests", "fixtures")
DEF = os.path.join(FIX, "deflate")
PNG = os.path.join(FIX, "png")
os.makedirs(DEF, exist_ok=True)
os.makedirs(PNG, exist_ok=True)

# --------------------------------------------------------------------------
# raw deflate streams
# --------------------------------------------------------------------------

def raw_deflate(data, level=6, strategy=zlib.Z_DEFAULT_STRATEGY):
    c = zlib.compressobj(level, zlib.DEFLATED, -15, 9, strategy)
    return c.compress(data) + c.flush()


deflate_cases = {}


def add_deflate(name, raw, uncompressed_len):
    deflate_cases[name] = (raw, uncompressed_len)


rnd = random.Random(0xC0FFEE)

payloads = {
    "empty": b"",
    "one": b"A",
    "three": b"abc",
    "four": b"abcd",
    "five": b"abcde",
    "seven": b"abcdefg",
    "hello": b"hello world",
    "runs": b"a" * 300,
    "runs2": (b"ab" * 500),
    "runs3": (b"abc" * 700),
    "text": (b"the quick brown fox jumps over the lazy dog. " * 40),
    "zeros": bytes(1024),
    "incr": bytes(range(256)) * 4,
    "rand64": bytes(rnd.randrange(256) for _ in range(64)),
    "rand1k": bytes(rnd.randrange(256) for _ in range(1024)),
    "rand4k": bytes(rnd.randrange(256) for _ in range(4096)),
    "mixed": (b"aaaaaaaaaabbbbbbbbbb" + bytes(rnd.randrange(256) for _ in range(200))
              + b"cccccccccc" * 30),
    "long_match": (b"X" * 5 + b"Y" * 259 + b"Z" * 1),
    "maxlen": (b"Q" * 258 + b"Q" * 258 + b"R"),
    "far": (bytes(rnd.randrange(256) for _ in range(40000)) ),
}

for pname, payload in payloads.items():
    for lvl, lname in ((0, "l0"), (1, "l1"), (6, "l6"), (9, "l9")):
        add_deflate(f"{pname}_{lname}", raw_deflate(payload, lvl), len(payload))
    add_deflate(f"{pname}_fixed", raw_deflate(payload, 6, zlib.Z_FIXED), len(payload))
    add_deflate(f"{pname}_huff", raw_deflate(payload, 6, zlib.Z_HUFFMAN_ONLY), len(payload))
    add_deflate(f"{pname}_rle", raw_deflate(payload, 6, zlib.Z_RLE), len(payload))

# Multi-block streams: concatenate several sync-flushed blocks then a final one.
for pname in ("text", "rand1k", "runs3"):
    payload = payloads[pname]
    c = zlib.compressobj(6, zlib.DEFLATED, -15)
    half = len(payload) // 2
    out = c.compress(payload[:half]) + c.flush(zlib.Z_SYNC_FLUSH)
    out += c.compress(payload[half:]) + c.flush()
    add_deflate(f"{pname}_multiblock", out, len(payload))

# --------------------------------------------------------------------------
# PNG files
# --------------------------------------------------------------------------

png_cases = []


def chunk(tag, data):
    return (struct.pack(">I", len(data)) + tag + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF))


SIG = b"\x89PNG\r\n\x1a\n"


def build_png(w, h, color_type, raw_rows, plte=None, trns=None,
              bit_depth=8, compression=0, filt=0, interlace=0,
              idat_split=1, level=6, strategy=zlib.Z_DEFAULT_STRATEGY,
              extra_chunks_before=(), sig=SIG, ihdr_w=None, ihdr_h=None):
    ihdr = struct.pack(">IIBBBBB",
                       w if ihdr_w is None else ihdr_w,
                       h if ihdr_h is None else ihdr_h,
                       bit_depth, color_type, compression, filt, interlace)
    out = sig + chunk(b"IHDR", ihdr)
    for c in extra_chunks_before:
        out += c
    if plte is not None:
        out += chunk(b"PLTE", plte)
    if trns is not None:
        out += chunk(b"tRNS", trns)
    raw = b"".join(raw_rows)
    c = zlib.compressobj(level, zlib.DEFLATED, 15, 9, strategy)
    z = c.compress(raw) + c.flush()
    if idat_split <= 1:
        out += chunk(b"IDAT", z)
    else:
        n = (len(z) + idat_split - 1) // idat_split
        if n == 0:
            n = 1
        for i in range(0, len(z), n):
            out += chunk(b"IDAT", z[i:i + n])
    out += chunk(b"IEND", b"")
    return out


BPP = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}


def paeth(a, b, c):
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    if pb <= pc:
        return b
    return c


def filter_rows(rows, bpp, ftype):
    """Apply PNG filter `ftype` to each scanline; returns list of raw rows."""
    out = []
    prev = bytes(len(rows[0]))
    for row in rows:
        if ftype == 0:
            f = row
        elif ftype == 1:
            f = bytes(((row[x] - (row[x - bpp] if x >= bpp else 0)) & 0xFF)
                      for x in range(len(row)))
        elif ftype == 2:
            f = bytes(((row[x] - prev[x]) & 0xFF) for x in range(len(row)))
        elif ftype == 3:
            f = bytearray(len(row))
            for x in range(len(row)):
                a = row[x - bpp] if x >= bpp else 0
                f[x] = (row[x] - ((a + prev[x]) // 2)) & 0xFF
            f = bytes(f)
        elif ftype == 4:
            f = bytearray(len(row))
            for x in range(len(row)):
                a = row[x - bpp] if x >= bpp else 0
                cc = prev[x - bpp] if x >= bpp else 0
                f[x] = (row[x] - paeth(a, prev[x], cc)) & 0xFF
            f = bytes(f)
        else:
            f = row
        out.append(bytes([ftype]) + f)
        prev = row
    return out


def make_rows(w, h, bpp, seed):
    r = random.Random(seed)
    rows = []
    for y in range(h):
        rows.append(bytes(r.randrange(256) for _ in range(w * bpp)))
    return rows


def add_png(name, data):
    png_cases.append(name)
    with open(os.path.join(PNG, name + ".png"), "wb") as fp:
        fp.write(data)


# 1. every colour type x every filter type x a few sizes
for ct in (0, 2, 3, 4, 6):
    bpp = BPP[ct]
    for (w, h) in ((1, 1), (1, 7), (7, 1), (3, 3), (8, 5), (16, 16), (33, 17), (64, 40)):
        for ft in (0, 1, 2, 3, 4):
            rows = make_rows(w, h, bpp, hash((ct, w, h, ft)) & 0xFFFF)
            raw = filter_rows(rows, bpp, ft)
            plte = trns = None
            if ct == 3:
                plte = bytes((i * 7 + 3) & 0xFF for i in range(256 * 3))
            add_png(f"ct{ct}_{w}x{h}_f{ft}",
                    build_png(w, h, ct, raw, plte=plte, trns=trns))

# 2. per-row varying filters
for ct in (0, 2, 3, 4, 6):
    bpp = BPP[ct]
    w, h = 13, 11
    rows = make_rows(w, h, bpp, 1234 + ct)
    raw = []
    prev = bytes(w * bpp)
    for y, row in enumerate(rows):
        ft = y % 5
        raw.append(filter_rows([row], bpp, ft)[0] if y == 0 else None)
        prev = row
    # simpler: rebuild honouring prev properly
    raw = []
    prev = bytes(w * bpp)
    for y, row in enumerate(rows):
        ft = y % 5
        if ft == 0:
            f = row
        elif ft == 1:
            f = bytes(((row[x] - (row[x - bpp] if x >= bpp else 0)) & 0xFF) for x in range(len(row)))
        elif ft == 2:
            f = bytes(((row[x] - prev[x]) & 0xFF) for x in range(len(row)))
        elif ft == 3:
            f = bytes(((row[x] - (((row[x - bpp] if x >= bpp else 0) + prev[x]) // 2)) & 0xFF) for x in range(len(row)))
        else:
            f = bytes(((row[x] - paeth((row[x - bpp] if x >= bpp else 0), prev[x], (prev[x - bpp] if x >= bpp else 0))) & 0xFF) for x in range(len(row)))
        raw.append(bytes([ft]) + f)
        prev = row
    plte = bytes((i * 11 + 5) & 0xFF for i in range(256 * 3)) if ct == 3 else None
    add_png(f"ct{ct}_mixedfilters", build_png(w, h, ct, raw, plte=plte))

# 3. palette variations: tRNS of various lengths, small palettes
for trns_len in (0, 1, 3, 16, 128, 256):
    w, h = 20, 9
    r = random.Random(99 + trns_len)
    rows = [bytes(r.randrange(256) for _ in range(w)) for _ in range(h)]
    raw = filter_rows(rows, 1, 0)
    plte = bytes((i * 5 + 1) & 0xFF for i in range(256 * 3))
    trns = bytes((i * 13 + 7) & 0xFF for i in range(trns_len))
    add_png(f"ct3_trns{trns_len}", build_png(w, h, 3, raw, plte=plte, trns=trns))

# palette-only (no tRNS chunk at all)
add_png("ct3_no_trns", build_png(6, 6, 3,
        filter_rows([bytes(range(6)) for _ in range(6)], 1, 0),
        plte=bytes((i * 3) & 0xFF for i in range(256 * 3))))

# 4. multiple IDAT chunks
for split in (2, 3, 5, 17):
    w, h = 24, 18
    rows = make_rows(w, h, 4, 555 + split)
    add_png(f"ct6_idat{split}", build_png(w, h, 6, filter_rows(rows, 4, 1),
                                          idat_split=split))
for split in (2, 4):
    w, h = 30, 12
    rows = make_rows(w, h, 3, 777 + split)
    add_png(f"ct2_idat{split}", build_png(w, h, 2, filter_rows(rows, 4, 2),
                                          idat_split=split))

# 5. compression levels / strategies
for lvl in (0, 1, 9):
    w, h = 40, 20
    rows = make_rows(w, h, 3, 4242 + lvl)
    add_png(f"ct2_level{lvl}", build_png(w, h, 2, filter_rows(rows, 3, 0), level=lvl))
for sname, strat in (("fixed", zlib.Z_FIXED), ("huff", zlib.Z_HUFFMAN_ONLY),
                     ("rle", zlib.Z_RLE)):
    w, h = 32, 16
    rows = make_rows(w, h, 4, 8080)
    add_png(f"ct6_{sname}", build_png(w, h, 6, filter_rows(rows, 4, 3),
                                      strategy=strat))

# solid colour images (long runs -> big distance-1 matches, memset path)
for ct in (0, 2, 4, 6):
    bpp = BPP[ct]
    w, h = 50, 30
    rows = [bytes([0x5A] * (w * bpp)) for _ in range(h)]
    add_png(f"ct{ct}_solid", build_png(w, h, ct, filter_rows(rows, bpp, 0)))

# larger image
add_png("ct6_big", build_png(128, 96, 6,
        filter_rows(make_rows(128, 96, 4, 31337), 4, 4)))
add_png("ct0_big", build_png(200, 150, 0,
        filter_rows(make_rows(200, 150, 1, 4711), 1, 1)))

# 6. chunks before PLTE / unknown ancillary chunks interleaved
add_png("ct3_with_gama", build_png(10, 10, 3,
        filter_rows([bytes(range(10)) for _ in range(10)], 1, 0),
        plte=bytes((i * 2) & 0xFF for i in range(256 * 3)),
        extra_chunks_before=(chunk(b"gAMA", struct.pack(">I", 45455)),)))
add_png("ct6_with_text", build_png(9, 9, 6,
        filter_rows(make_rows(9, 9, 4, 606), 4, 0),
        extra_chunks_before=(chunk(b"tEXt", b"Comment\x00hi there"),)))

# tRNS before PLTE ordering
w, h = 8, 8
rows = [bytes(range(8)) for _ in range(8)]
raw = filter_rows(rows, 1, 0)
ihdr = struct.pack(">IIBBBBB", w, h, 8, 3, 0, 0, 0)
z = zlib.compress(b"".join(raw), 6)
data = (SIG + chunk(b"IHDR", ihdr)
        + chunk(b"tRNS", bytes(range(64)))
        + chunk(b"PLTE", bytes((i * 9) & 0xFF for i in range(256 * 3)))
        + chunk(b"IDAT", z) + chunk(b"IEND", b""))
add_png("ct3_trns_before_plte", data)

# 7. error / edge cases
add_png("err_bad_sig", b"not a png file at all, really" + bytes(64))
add_png("err_bad_sig2", b"\x89PNG\r\n\x1a\x0a"[:7] + b"\x00" + bytes(64))
add_png("err_no_ihdr", SIG + chunk(b"XXXX", bytes(13)) + bytes(32))
add_png("err_short_ihdr", SIG + chunk(b"IHDR", bytes(12)) + bytes(32))
add_png("err_bitdepth", build_png(4, 4, 6, filter_rows(make_rows(4, 4, 4, 1), 4, 0),
                                  bit_depth=16))
add_png("err_bitdepth4", build_png(4, 4, 0, filter_rows(make_rows(4, 4, 1, 1), 1, 0),
                                   bit_depth=4))
add_png("err_colortype", build_png(4, 4, 5, filter_rows(make_rows(4, 4, 4, 1), 4, 0)))
add_png("err_colortype7", build_png(4, 4, 7, filter_rows(make_rows(4, 4, 4, 1), 4, 0)))
add_png("err_colortype1", build_png(4, 4, 1, filter_rows(make_rows(4, 4, 1, 1), 1, 0)))
add_png("err_h0", build_png(4, 4, 6, filter_rows(make_rows(4, 4, 4, 1), 4, 0),
                            ihdr_h=0))
add_png("err_compression", build_png(4, 4, 6, filter_rows(make_rows(4, 4, 4, 1), 4, 0),
                                     compression=1))
add_png("err_filter", build_png(4, 4, 6, filter_rows(make_rows(4, 4, 4, 1), 4, 0),
                                filt=1))
add_png("err_interlace", build_png(4, 4, 6, filter_rows(make_rows(4, 4, 4, 1), 4, 0),
                                   interlace=1))

# invalid filter byte inside the scanlines
rows = make_rows(6, 6, 4, 12)
raw = filter_rows(rows, 4, 0)
raw[3] = bytes([5]) + raw[3][1:]
add_png("err_filterbyte5", build_png(6, 6, 6, raw))
raw2 = filter_rows(rows, 4, 0)
raw2[0] = bytes([9]) + raw2[0][1:]
add_png("err_filterbyte9_row0", build_png(6, 6, 6, raw2))

# indexed image but no PLTE chunk
add_png("err_ct3_no_plte", build_png(5, 5, 3,
        filter_rows([bytes(range(5)) for _ in range(5)], 1, 0)))

# zlib header problems (IDAT payload hand-made)
def raw_idat_png(w, h, ct, payload, plte=None):
    ihdr = struct.pack(">IIBBBBB", w, h, 8, ct, 0, 0, 0)
    out = SIG + chunk(b"IHDR", ihdr)
    if plte is not None:
        out += chunk(b"PLTE", plte)
    out += chunk(b"IDAT", payload) + chunk(b"IEND", b"")
    return out

good_raw = b"".join(filter_rows(make_rows(4, 4, 4, 3), 4, 0))
z = zlib.compress(good_raw, 6)
add_png("err_zlib_cm", raw_idat_png(4, 4, 6, bytes([0x79]) + z[1:]))
add_png("err_zlib_window", raw_idat_png(4, 4, 6, bytes([0x88]) + z[1:]))
add_png("err_zlib_fdict", raw_idat_png(4, 4, 6, z[:1] + bytes([z[1] | 0x20]) + z[2:]))
add_png("err_zlib_short", raw_idat_png(4, 4, 6, z[:5]))
add_png("err_no_idat", SIG + chunk(b"IHDR", struct.pack(">IIBBBBB", 4, 4, 8, 6, 0, 0, 0))
        + chunk(b"IEND", b""))

# stored (uncompressed) deflate inside a PNG
zc = zlib.compressobj(0, zlib.DEFLATED, 15)
z0 = zc.compress(good_raw) + zc.flush()
add_png("ct6_stored", raw_idat_png(4, 4, 6, z0))

# --------------------------------------------------------------------------
# write everything out
# --------------------------------------------------------------------------
with open(os.path.join(DEF, "manifest.txt"), "w") as fp:
    for name in sorted(deflate_cases):
        raw, ulen = deflate_cases[name]
        with open(os.path.join(DEF, name + ".bin"), "wb") as b:
            b.write(raw)
        fp.write(f"{name} {ulen}\n")

with open(os.path.join(PNG, "manifest.txt"), "w") as fp:
    for name in png_cases:
        fp.write(name + "\n")

print(f"{len(deflate_cases)} deflate fixtures, {len(png_cases)} png fixtures")
