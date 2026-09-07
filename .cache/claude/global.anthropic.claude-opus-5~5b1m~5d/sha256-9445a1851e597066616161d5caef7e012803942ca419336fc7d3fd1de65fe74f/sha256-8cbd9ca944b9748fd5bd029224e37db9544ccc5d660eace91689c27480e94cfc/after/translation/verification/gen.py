#!/usr/bin/env python3
"""Generate a corpus of PNG files (valid + malformed) and raw DEFLATE streams."""
import os, struct, zlib, random, sys

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "corpus")
PNG = os.path.join(OUT, "png")
INF = os.path.join(OUT, "inflate")
for d in (PNG, INF):
    os.makedirs(d, exist_ok=True)

def chunk(typ, data):
    c = struct.pack(">I", len(data)) + typ + data
    return c + struct.pack(">I", zlib.crc32(typ + data) & 0xFFFFFFFF)

SIG = b"\x89PNG\r\n\x1a\n"

def ihdr(w, h, bd=8, ct=6, comp=0, filt=0, inter=0):
    return chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, bd, ct, comp, filt, inter))

def raw_rows(w, h, bpp, filters, seed=1):
    """Build filtered scanlines for an image of random-ish pixels."""
    rnd = random.Random(seed)
    img = [[rnd.randrange(256) for _ in range(w * bpp)] for _ in range(h)]
    out = bytearray()
    prev = [0] * (w * bpp)
    for y in range(h):
        f = filters[y % len(filters)]
        cur = img[y]
        out.append(f)
        line = bytearray()
        for x in range(w * bpp):
            a = cur[x - bpp] if x >= bpp else 0
            b = prev[x]
            c = prev[x - bpp] if x >= bpp else 0
            if f == 0:
                v = cur[x]
            elif f == 1:
                v = (cur[x] - a) & 0xFF
            elif f == 2:
                v = (cur[x] - b) & 0xFF
            elif f == 3:
                v = (cur[x] - ((a + b) >> 1)) & 0xFF
            else:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                v = (cur[x] - pr) & 0xFF
            line.append(v)
        out += line
        prev = cur
    return bytes(out)

def write(name, data):
    with open(os.path.join(PNG, name), "wb") as f:
        f.write(data)

def png(name, w, h, ct, filters=(0,), level=6, bd=8, comp=0, filt=0, inter=0,
        plte=None, trns=None, extra_before=b"", extra_after=b"", idat_split=1,
        raw_override=None, zlib_override=None, seed=1, trailing=b""):
    bpp = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}.get(ct, 1)
    if zlib_override is not None:
        z = zlib_override
    else:
        raw = raw_rows(w, h, bpp, list(filters), seed) if raw_override is None else raw_override
        z = zlib.compress(raw, level)
    body = SIG + ihdr(w, h, bd, ct, comp, filt, inter) + extra_before
    if plte is not None:
        body += chunk(b"PLTE", plte)
    if trns is not None:
        body += chunk(b"tRNS", trns)
    body += extra_after
    n = max(1, idat_split)
    step = (len(z) + n - 1) // n if z else 0
    if step == 0:
        body += chunk(b"IDAT", z)
    else:
        for i in range(0, len(z), step):
            body += chunk(b"IDAT", z[i:i + step])
    body += chunk(b"IEND", b"") + trailing
    write(name, body)

# ---------------------------------------------------------------- valid images
FILTERSETS = [(0,), (1,), (2,), (3,), (4,), (0, 1, 2, 3, 4), (4, 3, 2, 1, 0)]
for ct in (0, 2, 4, 6):
    for i, fs in enumerate(FILTERSETS):
        for (w, h) in ((1, 1), (2, 3), (7, 5), (16, 16), (37, 3), (1, 40)):
            for level in (0, 1, 6, 9):
                png(f"ok_ct{ct}_f{i}_{w}x{h}_l{level}.png", w, h, ct,
                    filters=fs, level=level, seed=w * 31 + h * 7 + ct + i)

# palette images
pal = bytes()
for i in range(256):
    pal += bytes(((i * 7) & 0xFF, (i * 13) & 0xFF, (i * 29) & 0xFF))
for i, fs in enumerate(FILTERSETS):
    for (w, h) in ((1, 1), (5, 4), (16, 16), (33, 2)):
        for tl in (None, 0, 1, 3, 256):
            trns = None if tl is None else bytes((j * 11) & 0xFF for j in range(tl))
            png(f"ok_ct3_f{i}_{w}x{h}_t{tl}.png", w, h, 3, filters=fs, level=6,
                plte=pal, trns=trns, seed=w + h * 3 + i)
# Short palette: indices past its end read out of bounds. A big padding chunk
# is appended right after PLTE so those reads stay inside the (deterministic)
# file buffer instead of hitting unrelated heap memory.
for n in (4, 32, 200):
    png(f"ok_ct3_shortpal{n}.png", 4, 4, 3, filters=(0,), plte=pal[:3 * n],
        extra_after=chunk(b"tEXt", b"pad\x00" + b"P" * 1600), seed=9)

# multiple IDAT chunks / chunk ordering / ancillary chunks
for n in (2, 3, 7, 33):
    png(f"ok_idatsplit{n}.png", 16, 9, 6, filters=(0, 1, 2, 3, 4), idat_split=n)
png("ok_extra_chunks.png", 8, 8, 2, filters=(1, 2),
    extra_before=chunk(b"gAMA", struct.pack(">I", 45455)),
    extra_after=chunk(b"tEXt", b"Comment\x00hello") + chunk(b"bKGD", b"\x00\x00\x00\x00\x00\x00"))
png("ok_plte_on_rgb.png", 8, 8, 2, filters=(0,), plte=pal)
png("ok_trns_on_rgb.png", 8, 8, 2, filters=(0,), trns=b"\x00\x10\x00\x20\x00\x30")
png("ok_trailing.png", 8, 8, 6, filters=(0,), trailing=b"junkjunkjunk")
png("ok_zero_width.png", 0, 4, 6, filters=(0,))
png("ok_big_1x1000.png", 1, 1000, 6, filters=(0, 1, 2, 3, 4))
png("ok_big_200x50.png", 200, 50, 2, filters=(0, 1, 2, 3, 4))
# stored (uncompressed) deflate blocks at several sizes -> exercises cp_stored
for (w, h) in ((1, 1), (3, 3), (16, 16), (17, 5), (64, 9)):
    png(f"ok_stored_{w}x{h}.png", w, h, 6, filters=(0, 1, 2, 3, 4), level=0)
# fixed-huffman blocks: highly repetitive data compresses with fixed trees
for (w, h) in ((8, 8), (32, 4)):
    png(f"ok_fixed_{w}x{h}.png", w, h, 6, filters=(0,), level=9,
        raw_override=bytes([0] * ((w * 4 + 1) * h)))
# zlib with window-size / FLEVEL variations
z = zlib.compress(raw_rows(4, 4, 4, [0], 5), 6)
for hdr in (b"\x08\x1d", b"\x18\x19", b"\x28\x15", b"\x38\x11", b"\x48\x0d",
            b"\x58\x09", b"\x68\x05", b"\x78\x01", b"\x78\x9c", b"\x78\xda"):
    png(f"ok_zhdr_{hdr.hex()}.png", 4, 4, 6, zlib_override=hdr + z[2:])

# -------------------------------------------------------------- invalid images
write("bad_empty.bin", b"")
write("bad_short1.bin", b"\x89")
write("bad_short7.bin", SIG[:7])
write("bad_sig.bin", b"\x89PNGxxxx" + ihdr(4, 4))
write("bad_sig2.bin", b"BADSIGNATURE")
write("bad_no_ihdr.bin", SIG + chunk(b"IDAT", b"\x78\x9c\x03\x00\x00\x00\x00\x01"))
write("bad_ihdr_short.bin", SIG + chunk(b"IHDR", b"\x00" * 12))
write("bad_ihdr_truncated.bin", SIG + struct.pack(">I", 13) + b"IHDR" + b"\x00" * 5)
for bd in (1, 2, 4, 16):
    png(f"bad_bitdepth{bd}.png", 4, 4, 6, bd=bd)
for ct in (1, 5, 7, 255):
    png(f"bad_colortype{ct}.png", 4, 4, ct)
png("bad_height0.png", 4, 0, 6)
SMALL = zlib.compress(b"\x00" * 64, 6)
png("bad_huge_w.png", 0x20000000, 1, 6, zlib_override=SMALL)
png("bad_huge_h.png", 4, 0x20000000, 6, zlib_override=SMALL)
png("bad_huge_both.png", 0xFFFFFFFF, 0xFFFFFFFF, 6, zlib_override=SMALL)
png("bad_w_wraps.png", 0xFFFFFFFF, 4, 6, zlib_override=SMALL)
png("bad_w_wraps2.png", 0x7FFFFFFF, 4, 6, zlib_override=SMALL)
png("bad_compression.png", 4, 4, 6, comp=1)
png("bad_filter.png", 4, 4, 6, filt=1)
png("bad_interlace.png", 4, 4, 6, inter=1)
write("bad_no_idat.bin", SIG + ihdr(4, 4) + chunk(b"IEND", b""))
png("bad_zlib_short.png", 4, 4, 6, zlib_override=b"\x78\x9c\x01")
png("bad_zlib_cm.png", 4, 4, 6, zlib_override=b"\x79" + z[1:])
png("bad_zlib_window.png", 4, 4, 6, zlib_override=b"\x88" + z[1:])
png("bad_zlib_preset.png", 4, 4, 6, zlib_override=b"\x78\x3d" + z[2:])
png("bad_deflate_truncated.png", 16, 16, 6, zlib_override=z[:8])
png("bad_deflate_btype3.png", 4, 4, 6, zlib_override=b"\x78\x9c\x07\x00\x00\x00\x00\x00")
png("bad_filterbyte.png", 4, 4, 6, filters=(0,),
    raw_override=b"\x09" + bytes(range(16)) * 4 + b"\x00" * 3)
png("bad_ct3_noplte.png", 4, 4, 3, filters=(0,))
png("bad_idat_overlong.png", 4, 4, 6, filters=(0,),
    raw_override=bytes([7] * 4096))
png("bad_stored_overlong.png", 2, 2, 6, filters=(0,), level=0,
    raw_override=bytes([3] * 1024))

# chunk-length edge cases -------------------------------------------------
def rawchunk(length, typ, data):
    """chunk with an arbitrary declared length field"""
    return struct.pack(">I", length) + typ + data + struct.pack(">I", 0)

zz = zlib.compress(raw_rows(4, 4, 4, [0], 3), 6)
# IHDR whose declared length is 0x80000000 -> cp_chunk's `int offset` goes
# negative and png->p walks backwards
write("bad_ihdr_len_negative.bin",
      SIG + rawchunk(0x80000000, b"IHDR", struct.pack(">IIBBBBB", 4, 4, 8, 6, 0, 0, 0))
      + chunk(b"IDAT", zz) + chunk(b"IEND", b""))
# a chunk whose declared length wraps in cp_find (len + 12 == 11)
write("bad_chunk_len_wrap.bin",
      SIG + ihdr(4, 4) + rawchunk(0xFFFFFFFF, b"junk", b"") + chunk(b"IDAT", zz)
      + chunk(b"IEND", b""))
# IDAT with a huge declared length -> datalen becomes negative
write("bad_idat_len_huge.bin",
      SIG + ihdr(4, 4) + rawchunk(0xFFFFFFFF, b"IDAT", zz) + chunk(b"IEND", b""))
write("bad_idat_len_big.bin",
      SIG + ihdr(4, 4) + rawchunk(0x7FFFFFFF, b"IDAT", zz) + chunk(b"IEND", b""))
# PLTE / tRNS after IDAT (the IDAT scan starts after whichever was found)
write("ok_plte_after_idat.bin",
      SIG + ihdr(4, 4, 8, 3) + chunk(b"IDAT", zlib.compress(bytes([0, 1, 2, 3, 0] * 4), 6))
      + chunk(b"PLTE", pal) + chunk(b"IEND", b""))
write("ok_trns_before_plte.bin",
      SIG + ihdr(4, 4, 8, 3) + chunk(b"tRNS", b"\x11\x22\x33\x44")
      + chunk(b"PLTE", pal)
      + chunk(b"IDAT", zlib.compress(bytes([0, 1, 2, 3, 0] * 4), 6))
      + chunk(b"IEND", b""))
write("ok_idat_before_plte.bin",
      SIG + ihdr(4, 4, 8, 3)
      + chunk(b"IDAT", zlib.compress(bytes([0, 1, 2, 3, 0] * 4), 6))
      + chunk(b"PLTE", pal)
      + chunk(b"IDAT", zlib.compress(bytes([1, 1, 1, 1, 1] * 4), 6))
      + chunk(b"IEND", b""))
# invalid filter byte on a row other than the first
for row in (1, 2, 3):
    body = bytearray()
    for y in range(4):
        body.append(9 if y == row else 0)
        body += bytes([y * 16 + x for x in range(16)])
    png(f"bad_filterbyte_row{row}.png", 4, 4, 6, raw_override=bytes(body))
for fb in (5, 6, 200, 255):
    body = bytearray()
    for y in range(4):
        body.append(fb if y == 2 else 0)
        body += bytes([y * 16 + x for x in range(16)])
    png(f"bad_filterbyte_v{fb}.png", 4, 4, 6, raw_override=bytes(body))
# IEND missing entirely / IDAT is the last chunk
write("ok_no_iend.bin", SIG + ihdr(4, 4) + chunk(b"IDAT", zz))
# zero-length IDAT chunks mixed in
write("ok_empty_idats.bin",
      SIG + ihdr(4, 4) + chunk(b"IDAT", b"") + chunk(b"IDAT", zz)
      + chunk(b"IDAT", b"") + chunk(b"IEND", b""))

# ------------------------------------------------------- raw DEFLATE fragments
def winf(name, data):
    with open(os.path.join(INF, name), "wb") as f:
        f.write(data)

samples = {
    "empty": b"",
    "one": b"\x00",
    "stored_empty": zlib.compress(b"", 0)[2:-4],
    "stored_hello": zlib.compress(b"hello world", 0)[2:-4],
    "stored_256": zlib.compress(bytes(range(256)), 0)[2:-4],
    "fixed_zeros": zlib.compress(bytes(300), 9)[2:-4],
    "dyn_text": zlib.compress(b"the quick brown fox jumps over the lazy dog " * 20, 9)[2:-4],
    "dyn_rand": zlib.compress(bytes(random.Random(7).randrange(256) for _ in range(500)), 9)[2:-4],
    "multi_block": (zlib.compress(b"aaaaaaaaaaaaaaaaaaaa", 0)[2:-4]),
    "trunc_dyn": zlib.compress(b"abcdefgh" * 40, 9)[2:-4][:6],
    "garbage1": bytes([0xFF] * 16),
    "garbage2": bytes([0x55] * 33),
    "garbage3": bytes([0x00] * 9),
    "garbage4": b"\x02\x00\x00\x00\x00",
    "rle": zlib.compress(b"x" * 1000, 9)[2:-4],
    "long_match": zlib.compress((b"abcd" * 500), 9)[2:-4],
}

class BW:
    """DEFLATE bit writer."""
    def __init__(self):
        self.bits = []
    def wb(self, v, n):          # value, LSB first (extra bits / headers)
        for i in range(n):
            self.bits.append((v >> i) & 1)
    def wc(self, v, n):          # huffman code, MSB first
        for i in range(n - 1, -1, -1):
            self.bits.append((v >> i) & 1)
    def out(self):
        b = bytearray()
        for i in range(0, len(self.bits), 8):
            v = 0
            for j, bit in enumerate(self.bits[i:i + 8]):
                v |= bit << j
            b.append(v)
        return bytes(b)

def fixed_lit(bw, sym):
    if sym < 144:
        bw.wc(0x30 + sym, 8)
    elif sym < 256:
        bw.wc(0x190 + sym - 144, 9)
    elif sym < 280:
        bw.wc(sym - 256, 7)
    else:
        bw.wc(0xC0 + sym - 280, 8)

# stored block whose LEN is smaller than the remaining input
bw = BW(); bw.wb(0, 1); bw.wb(0, 2)
s = bw.out() + b"\x00\x00\xff\xff" + b"\xAB" * 40
winf("stored_len_too_small", s)
bw = BW(); bw.wb(0, 1); bw.wb(0, 2)
winf("stored_len_ok_then_eof", bw.out() + b"\x02\x00\xfd\xff" + b"hi")
# LEN/NLEN not complements
bw = BW(); bw.wb(0, 1); bw.wb(0, 2)
winf("stored_bad_nlen", bw.out() + b"\x04\x00\x00\x00" + b"abcd")

# fixed-huffman: back reference before the start of the output buffer
bw = BW(); bw.wb(1, 1); bw.wb(1, 2)
fixed_lit(bw, 65)
fixed_lit(bw, 257)       # length 3
bw.wc(5, 5)              # distance code 5 -> distance 7
fixed_lit(bw, 256)
winf("fixed_bad_distance", bw.out())

# fixed-huffman exercising 9-bit literals, 8-bit high literals, long
# lengths/distances with extra bits, and distance 1 (memset path)
bw = BW(); bw.wb(1, 1); bw.wb(1, 2)
for c in (0, 1, 143, 144, 200, 255):
    fixed_lit(bw, c)
fixed_lit(bw, 257); bw.wc(0, 5)                 # len 3, dist 1 -> memset path
fixed_lit(bw, 285); bw.wc(4, 5); bw.wb(0, 1)    # len 258, dist code 4 -> 5..6
fixed_lit(bw, 269); bw.wb(2, 2)                 # len 19+2
bw.wc(9, 5); bw.wb(3, 3)                        # dist code 9 -> 25 + 3
fixed_lit(bw, 284); bw.wb(30, 5)                # len 227+30 = 257
bw.wc(29, 5); bw.wb(4095, 13)                   # dist code 29 -> 24577+4095
fixed_lit(bw, 256)
winf("fixed_mix", bw.out())

# two fixed blocks, the first not final
bw = BW()
bw.wb(0, 1); bw.wb(1, 2)
for c in b"hello ":
    fixed_lit(bw, c)
fixed_lit(bw, 256)
bw.wb(1, 1); bw.wb(1, 2)
for c in b"world":
    fixed_lit(bw, c)
fixed_lit(bw, 256)
winf("fixed_two_blocks", bw.out())

# dynamic block with an all-zero code-length alphabet -> empty huffman trees,
# which makes cp_decode read tree[-1]
bw = BW(); bw.wb(1, 1); bw.wb(2, 2)
bw.wb(0, 5); bw.wb(0, 5); bw.wb(0, 4)
for _ in range(4):
    bw.wb(0, 3)
winf("dyn_empty_trees", bw.out() + b"\x00" * 8)

# dynamic block using the 16/17/18 run-length code-length symbols
bw = BW(); bw.wb(1, 1); bw.wb(2, 2)
bw.wb(0, 5); bw.wb(0, 5); bw.wb(15, 4)   # nlit=257 ndst=1 nlen=19
lens = [0] * 19
lens[0] = 2   # symbol 0
lens[1] = 2   # symbol 1
lens[17] = 2  # 3-10 zeros
lens[18] = 2  # 11-138 zeros
for i in (16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15):
    bw.wb(lens[i], 3)
# code-length huffman: symbols 0,1,17,18 all length 2 -> codes 00,01,10,11
cl = {0: 0b00, 1: 0b01, 17: 0b10, 18: 0b11}
def emit_cl(sym, extra=None, nextra=0):
    bw.wc(cl[sym], 2)
    if extra is not None:
        bw.wb(extra, nextra)
emit_cl(1)              # lit 0 length 1
emit_cl(1)              # lit 1 length 1
emit_cl(18, 127, 7)     # 138 zeros
emit_cl(18, 116, 7)     # 127 zeros -> 267
emit_cl(0)
emit_cl(17, 7, 3)       # 10 zeros -> 278 ... pad out to 258 entries
winf("dyn_runlength", bw.out() + b"\x00" * 16)

for k, v in samples.items():
    winf(k, v)

print("png:", len(os.listdir(PNG)), "inflate:", len(os.listdir(INF)))
