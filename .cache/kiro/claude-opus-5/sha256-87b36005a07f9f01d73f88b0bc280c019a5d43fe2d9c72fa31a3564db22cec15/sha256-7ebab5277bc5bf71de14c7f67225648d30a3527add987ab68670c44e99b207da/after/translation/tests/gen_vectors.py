#!/usr/bin/env python3
"""Generate raw DEFLATE test vectors for the C-vs-Rust differential tests.

Writes `tests/data/<name>.deflate` files.  The tests only compare the C and
Rust outputs against each other, so no expected output is stored; the raw
length is embedded in the file name suffix `.rawlen` for buffer sizing.
"""
import os
import zlib
import random
import struct

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "data")
os.makedirs(DATA, exist_ok=True)

for f in os.listdir(DATA):
    os.remove(os.path.join(DATA, f))

manifest = []


def emit(name, deflate, rawlen):
    path = os.path.join(DATA, name + ".deflate")
    with open(path, "wb") as fh:
        fh.write(deflate)
    manifest.append((name, len(deflate), rawlen))


def raw_deflate(data, level=6, strategy=zlib.Z_DEFAULT_STRATEGY):
    c = zlib.compressobj(level, zlib.DEFLATED, -15, 9, strategy)
    return c.compress(data) + c.flush()


# ---------------------------------------------------------------- zlib streams
cases = {
    "empty": b"",
    "one_byte": b"A",
    "two_bytes": b"Hi",
    "short_ascii": b"hello world",
    "repeat_a": b"A" * 300,
    "repeat_ab": b"AB" * 500,
    "lorem": (b"the quick brown fox jumps over the lazy dog. " * 40),
    "binary_seq": bytes(range(256)) * 4,
    "zeros": bytes(1000),
    "text_mix": (b"aaaaaaaaaabbbbbbbbbbccccccccccdddddddddd" * 25
                 + b"0123456789" * 30),
}
random.seed(0xC0FFEE)
cases["random_512"] = bytes(random.randrange(256) for _ in range(512))
cases["random_5000"] = bytes(random.randrange(256) for _ in range(5000))
cases["low_entropy"] = bytes(random.choice(b"abc") for _ in range(4000))
cases["dist_stress"] = b"".join(
    bytes([random.randrange(4)]) * random.randrange(1, 40) for _ in range(600)
)
cases["long_match"] = b"x" * 70000
cases["big_text"] = (b"deflate test payload " * 4000)

for name, data in cases.items():
    for level in (0, 1, 6, 9):
        emit("%s_l%d" % (name, level), raw_deflate(data, level), len(data))
    # Huffman-only and RLE strategies exercise different block types.
    emit("%s_huff" % name, raw_deflate(data, 9, zlib.Z_HUFFMAN_ONLY), len(data))
    emit("%s_rle" % name, raw_deflate(data, 9, zlib.Z_RLE), len(data))
    emit("%s_fixed" % name, raw_deflate(data, 9, zlib.Z_FIXED), len(data))

# Multi-block streams via explicit SYNC_FLUSH boundaries.
for name, data in (("multi_small", b"chunk-a" * 20), ("multi_big", b"payload!" * 900)):
    c = zlib.compressobj(9, zlib.DEFLATED, -15)
    out = b""
    n = max(1, len(data) // 3)
    for i in range(0, len(data), n):
        out += c.compress(data[i:i + n])
        out += c.flush(zlib.Z_SYNC_FLUSH)
    out += c.flush(zlib.Z_FINISH)
    emit(name, out, len(data))


# ------------------------------------------------------- hand crafted streams
class BW:
    def __init__(self):
        self.acc = 0
        self.n = 0
        self.buf = bytearray()

    def bits(self, value, count):
        """LSB-first (deflate integer fields)."""
        for i in range(count):
            self.acc |= ((value >> i) & 1) << self.n
            self.n += 1
            if self.n == 8:
                self.buf.append(self.acc)
                self.acc = 0
                self.n = 0

    def code(self, value, count):
        """MSB-first (deflate Huffman codes)."""
        for i in reversed(range(count)):
            self.bits((value >> i) & 1, 1)

    def align(self):
        if self.n:
            self.buf.append(self.acc)
            self.acc = 0
            self.n = 0

    def done(self):
        self.align()
        return bytes(self.buf)


def fixed_lit(bw, sym):
    if sym <= 143:
        bw.code(0x30 + sym, 8)
    elif sym <= 255:
        bw.code(0x190 + sym - 144, 9)
    elif sym <= 279:
        bw.code(sym - 256, 7)
    else:
        bw.code(0xC0 + sym - 280, 8)


LEN_BASE = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43,
            51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258]
LEN_EXTRA = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4,
             4, 4, 5, 5, 5, 5, 0]
DIST_BASE = [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257,
             385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289,
             16385, 24577]
DIST_EXTRA = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9,
              10, 10, 11, 11, 12, 12, 13, 13]


def fixed_stream(ops, bfinal=1):
    """ops: ('lit', b) or ('match', length, distance)."""
    bw = BW()
    bw.bits(bfinal, 1)
    bw.bits(1, 2)  # btype = 1 (fixed)
    produced = 0
    for op in ops:
        if op[0] == "lit":
            fixed_lit(bw, op[1])
            produced += 1
        else:
            _, length, dist = op
            ls = max(i for i in range(29) if LEN_BASE[i] <= length)
            fixed_lit(bw, 257 + ls)
            bw.bits(length - LEN_BASE[ls], LEN_EXTRA[ls])
            ds = max(i for i in range(30) if DIST_BASE[i] <= dist)
            bw.code(ds, 5)
            bw.bits(dist - DIST_BASE[ds], DIST_EXTRA[ds])
            produced += length
    fixed_lit(bw, 256)
    return bw.done(), produced


# every length symbol, distance 1 (memset path) and distance 2 (byte loop)
for dist, tag in ((1, "d1"), (2, "d2"), (3, "d3")):
    ops = [("lit", 65), ("lit", 66), ("lit", 67), ("lit", 68)]
    total = 4
    for i in range(29):
        ln = LEN_BASE[i]
        ops.append(("match", ln, dist))
        total += ln
    s, produced = fixed_stream(ops)
    emit("crafted_all_len_%s" % tag, s, produced)

# every distance symbol, seeded with enough literals
ops = [("lit", 32 + (i % 90)) for i in range(30000)]
total = 30000
for i in range(30):
    ops.append(("match", 3 + (i % 20), DIST_BASE[i]))
    total += 3 + (i % 20)
s, produced = fixed_stream(ops)
emit("crafted_all_dist", s, produced)

# literals covering the whole 0..287 literal alphabet range that is valid
ops = [("lit", i) for i in range(256)]
s, produced = fixed_stream(ops)
emit("crafted_all_literals", s, produced)

# multiple fixed blocks
bw = BW()
for blk in range(4):
    bw.bits(1 if blk == 3 else 0, 1)
    bw.bits(1, 2)
    for i in range(50):
        fixed_lit(bw, 97 + ((blk * 7 + i) % 26))
    fixed_lit(bw, 256)
emit("crafted_multi_fixed", bw.done(), 200)

# stored blocks, hand written
def stored_stream(chunks, trailing=0):
    bw = BW()
    for i, ch in enumerate(chunks):
        bw.bits(1 if i == len(chunks) - 1 else 0, 1)
        bw.bits(0, 2)
        bw.align()
        bw.buf += struct.pack("<HH", len(ch), (~len(ch)) & 0xFFFF)
        bw.buf += ch
    out = bw.done()
    return out + b"\x00" * trailing


emit("crafted_stored_one", stored_stream([b"stored payload here"]), 19)
emit("crafted_stored_multi",
     stored_stream([b"aaa", b"bbbbbb", b"c" * 100]), 3 + 6 + 100)
emit("crafted_stored_empty", stored_stream([b""]), 0)
emit("crafted_stored_big", stored_stream([bytes(range(256)) * 40]), 256 * 40)

# mixed stored + fixed
bw = BW()
bw.bits(0, 1)
bw.bits(0, 2)
bw.align()
bw.buf += struct.pack("<HH", 5, (~5) & 0xFFFF)
bw.buf += b"first"
bw.bits(1, 1)
bw.bits(1, 2)
for c in b"second":
    fixed_lit(bw, c)
fixed_lit(bw, 256)
emit("crafted_mixed", bw.done(), 11)

# ------------------------------------------------- randomized length sweep
# Exercises every `in_bytes % 4` / `first_bytes` / `final_word` combination in
# cp_inflate's word splitting, plus lots of distinct Huffman tables.
rng2 = random.Random(0xBADC0DE)
alphabets = [b"a", b"ab", b"abc", bytes(range(16)), bytes(range(256)),
             b"the quick brown fox "]
sweep = 0
for length in list(range(1, 40)) + [61, 62, 63, 64, 65, 66, 127, 128, 129,
                                    255, 256, 257, 1021, 1024, 1027]:
    alpha = alphabets[length % len(alphabets)]
    payload = bytes(alpha[rng2.randrange(len(alpha))] for _ in range(length))
    for level in (0, 1, 9):
        emit("sweep%03d_l%d" % (length, level), raw_deflate(payload, level),
             length)
        sweep += 1
    emit("sweep%03d_fx" % length, raw_deflate(payload, 9, zlib.Z_FIXED), length)
    emit("sweep%03d_hf" % length, raw_deflate(payload, 9, zlib.Z_HUFFMAN_ONLY),
         length)

# Streams with trailing garbage appended (the decoder must stop at BFINAL).
for base in ("short_ascii_l9", "lorem_l6", "crafted_all_len_d1"):
    src = open(os.path.join(DATA, base + ".deflate"), "rb").read()
    rl = dict((n, r) for n, _, r in manifest)[base]
    emit("trail_" + base, src + bytes(rng2.randrange(256) for _ in range(37)), rl)

# ==========================================================================
# A real dynamic-Huffman encoder.  zlib only ever emits "reasonable" trees;
# this produces randomised but *valid* ones (deep 15-bit codes, degenerate
# single-code distance trees, minimum NLEN, heavy 16/17/18 RLE use) to stress
# cp_build / cp_decode / cp_dynamic.
# ==========================================================================
import heapq

PERM = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15]


def huffman_lengths(freqs, maxlen=15):
    """Canonical code lengths for {sym: freq}; retries until all <= maxlen."""
    syms = sorted(freqs)
    if len(syms) == 1:
        return {syms[0]: 1}
    weights = dict((s, freqs[s]) for s in syms)
    for attempt in range(60):
        heap = []
        for i, s in enumerate(syms):
            heapq.heappush(heap, (weights[s], i, [s]))
        depth = dict((s, 0) for s in syms)
        counter = len(syms)
        while len(heap) > 1:
            w1, _, l1 = heapq.heappop(heap)
            w2, _, l2 = heapq.heappop(heap)
            for s in l1 + l2:
                depth[s] += 1
            heapq.heappush(heap, (w1 + w2, counter, l1 + l2))
            counter += 1
        if max(depth.values()) <= maxlen:
            return depth
        # flatten the weight distribution and retry
        weights = dict((s, max(1, weights[s]) + attempt * 4) for s in syms)
    # fall back to a balanced code
    import math
    n = len(syms)
    b = int(math.ceil(math.log2(n)))
    return dict((s, b) for s in syms)


def canonical_codes(lengths):
    """{sym: length} -> {sym: (code, length)} using the DEFLATE rule."""
    bl_count = {}
    for s, l in lengths.items():
        if l:
            bl_count[l] = bl_count.get(l, 0) + 1
    code = 0
    next_code = {}
    for bits in range(1, 16):
        code = (code + bl_count.get(bits - 1, 0)) << 1
        next_code[bits] = code
    out = {}
    for s in sorted(lengths):
        l = lengths[s]
        if l:
            out[s] = (next_code[l], l)
            next_code[l] += 1
    return out


def rle_code_lengths(seq, rng):
    """Encode a code-length vector with the 0..18 alphabet."""
    ops = []
    i = 0
    n = len(seq)
    while i < n:
        v = seq[i]
        run = 1
        while i + run < n and seq[i + run] == v:
            run += 1
        if v == 0:
            while run >= 11 and rng.random() < 0.9:
                take = min(138, run)
                ops.append((18, take - 11, 7))
                run -= take
                i += take
            while run >= 3 and rng.random() < 0.9:
                take = min(10, run)
                ops.append((17, take - 3, 3))
                run -= take
                i += take
            for _ in range(run):
                ops.append((0, 0, 0))
                i += 1
        else:
            ops.append((v, 0, 0))
            i += 1
            run -= 1
            while run >= 3 and rng.random() < 0.9:
                take = min(6, run)
                ops.append((16, take - 3, 2))
                run -= take
                i += take
            for _ in range(run):
                ops.append((v, 0, 0))
                i += 1
    return ops


def encode_dynamic(bw, tokens, rng, bfinal=1):
    """tokens: ('lit', byte) | ('match', length, distance).  Returns raw size."""
    lit_freq = {256: 1}
    dst_freq = {}
    produced = 0
    encoded = []
    for t in tokens:
        if t[0] == "lit":
            lit_freq[t[1]] = lit_freq.get(t[1], 0) + rng.randrange(1, 400)
            encoded.append(("lit", t[1]))
            produced += 1
        else:
            _, ln, dist = t
            ls = max(i for i in range(29) if LEN_BASE[i] <= ln)
            ds = max(i for i in range(30) if DIST_BASE[i] <= dist)
            sym = 257 + ls
            lit_freq[sym] = lit_freq.get(sym, 0) + rng.randrange(1, 400)
            dst_freq[ds] = dst_freq.get(ds, 0) + rng.randrange(1, 400)
            encoded.append(("match", ls, ln - LEN_BASE[ls], ds, dist - DIST_BASE[ds]))
            produced += ln
    if not dst_freq:
        dst_freq = {0: 1}

    lit_len = huffman_lengths(lit_freq)
    dst_len = huffman_lengths(dst_freq)
    nlit = max(257, max(lit_len) + 1)
    ndst = max(1, max(dst_len) + 1)
    lit_vec = [lit_len.get(i, 0) for i in range(nlit)]
    dst_vec = [dst_len.get(i, 0) for i in range(ndst)]

    ops = rle_code_lengths(lit_vec + dst_vec, rng)
    cl_freq = {}
    for sym, _, _ in ops:
        cl_freq[sym] = cl_freq.get(sym, 0) + rng.randrange(1, 200)
    cl_len = huffman_lengths(cl_freq, maxlen=7)
    lenlens = [cl_len.get(i, 0) for i in range(19)]
    nlen = 4
    for idx in range(19):
        if lenlens[PERM[idx]]:
            nlen = max(nlen, idx + 1)

    lit_codes = canonical_codes(dict((i, lit_vec[i]) for i in range(nlit)))
    dst_codes = canonical_codes(dict((i, dst_vec[i]) for i in range(ndst)))
    cl_codes = canonical_codes(dict((i, lenlens[i]) for i in range(19)))

    bw.bits(bfinal, 1)
    bw.bits(2, 2)
    bw.bits(nlit - 257, 5)
    bw.bits(ndst - 1, 5)
    bw.bits(nlen - 4, 4)
    for idx in range(nlen):
        bw.bits(lenlens[PERM[idx]], 3)
    for sym, extra, nbits in ops:
        c, l = cl_codes[sym]
        bw.code(c, l)
        if nbits:
            bw.bits(extra, nbits)
    for e in encoded:
        if e[0] == "lit":
            c, l = lit_codes[e[1]]
            bw.code(c, l)
        else:
            _, ls, lextra, ds, dextra = e
            c, l = lit_codes[257 + ls]
            bw.code(c, l)
            if LEN_EXTRA[ls]:
                bw.bits(lextra, LEN_EXTRA[ls])
            c, l = dst_codes[ds]
            bw.code(c, l)
            if DIST_EXTRA[ds]:
                bw.bits(dextra, DIST_EXTRA[ds])
    c, l = lit_codes[256]
    bw.code(c, l)
    return produced


def random_tokens(rng, count, alphabet):
    toks = []
    produced = 0
    for _ in range(count):
        if produced >= 4 and rng.random() < 0.45:
            dist = rng.randrange(1, min(produced, 32768) + 1)
            ln = LEN_BASE[rng.randrange(29)] + rng.randrange(0, 3)
            ln = min(ln, 258)
            toks.append(("match", ln, dist))
            produced += ln
        else:
            toks.append(("lit", rng.choice(alphabet)))
            produced += 1
    return toks


frng = random.Random(0xF122ED)
ALPHAS = [
    list(range(256)),
    [0, 255],
    list(range(0, 256, 17)),
    [65],
    list(range(0, 32)),
    [1, 2, 3, 250, 251, 252, 253, 254, 255],
]
for case in range(70):
    alpha = ALPHAS[case % len(ALPHAS)]
    count = [1, 2, 3, 5, 9, 17, 40, 120, 400, 1500][case % 10]
    toks = random_tokens(frng, count, alpha)
    bw = BW()
    produced = encode_dynamic(bw, toks, frng)
    emit("dyn%03d" % case, bw.done(), produced)

# multi-block dynamic streams
for case in range(12):
    bw = BW()
    total = 0
    nblocks = 1 + case % 4
    for b in range(nblocks):
        toks = random_tokens(frng, 1 + frng.randrange(200),
                             ALPHAS[(case + b) % len(ALPHAS)])
        total += encode_dynamic(bw, toks, frng, bfinal=1 if b == nblocks - 1 else 0)
    emit("dynmulti%02d" % case, bw.done(), total)

# dynamic blocks interleaved with fixed and stored blocks
for case in range(10):
    bw = BW()
    total = 0
    kinds = [frng.choice("df") for _ in range(1 + case % 3)] + ["d"]
    for i, k in enumerate(kinds):
        final = 1 if i == len(kinds) - 1 else 0
        if k == "d":
            toks = random_tokens(frng, 1 + frng.randrange(120), ALPHAS[i % len(ALPHAS)])
            total += encode_dynamic(bw, toks, frng, bfinal=final)
        else:
            bw.bits(final, 1)
            bw.bits(1, 2)
            for _ in range(1 + frng.randrange(60)):
                fixed_lit(bw, frng.randrange(256))
                total += 1
            fixed_lit(bw, 256)
    emit("dynmix%02d" % case, bw.done(), total)

# ------------------------------------------------------------- error vectors
# btype == 3
bw = BW()
bw.bits(1, 1)
bw.bits(3, 2)
emit("err_btype3", bw.done() + b"\x00" * 16, 0)

# stored block with LEN/NLEN not complements
bw = BW()
bw.bits(1, 1)
bw.bits(0, 2)
bw.align()
bw.buf += struct.pack("<HH", 4, 0x1234)
bw.buf += b"abcd"
emit("err_stored_nlen", bw.done(), 4)

# stored block followed by extra input (trips the "extends beyond end" check)
emit("err_stored_beyond", stored_stream([b"abcd"], trailing=64), 4)

# invalid backwards distance: one literal then a match at distance 5
s, produced = fixed_stream([("lit", 65), ("match", 3, 5)])
emit("err_bad_distance", s, produced)

# distance reaching exactly to the start (valid) vs one past (invalid)
s, produced = fixed_stream([("lit", 65), ("lit", 66), ("match", 3, 2)])
emit("err_distance_edge_ok", s, produced)
s, produced = fixed_stream([("lit", 65), ("lit", 66), ("match", 3, 3)])
emit("err_distance_edge_bad", s, produced)

with open(os.path.join(DATA, "manifest.txt"), "w") as fh:
    for name, dlen, rawlen in sorted(manifest):
        fh.write("%s %d %d\n" % (name, dlen, rawlen))

print("wrote %d vectors" % len(manifest))
