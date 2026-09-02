#!/usr/bin/env python3
"""Generate a deterministic corpus of real zlib raw-DEFLATE streams.

Output format (little endian):
    u32 n_records
    then per record: u32 raw_len, u32 comp_len, raw bytes, comp bytes
"""
import random
import struct
import sys
import zlib

random.seed(0x9E3779B9)

payloads = []

# empty and tiny
payloads.append(b"")
for n in range(1, 9):
    payloads.append(bytes(random.randrange(256) for _ in range(n)))

# random incompressible data of various sizes
for n in (16, 31, 32, 33, 63, 64, 65, 127, 255, 256, 257, 1000, 4096):
    payloads.append(bytes(random.randrange(256) for _ in range(n)))

# highly repetitive (forces long matches, dist == 1 and large distances)
payloads.append(b"a" * 300)
payloads.append(b"ab" * 500)
payloads.append(b"abcdefgh" * 400)
payloads.append((b"the quick brown fox jumps over the lazy dog. " * 80))
payloads.append(bytes([0]) * 5000)
payloads.append(bytes(range(256)) * 40)

# text-like with skewed byte frequencies (drives deep Huffman codes)
alphabet = b"abcdefghijklmnopqrstuvwxyz \n.,"
weights = [max(1, 40 - i) for i in range(len(alphabet))]
for n in (500, 3000, 12000):
    payloads.append(bytes(random.choices(alphabet, weights=weights, k=n)))

# long-distance repeats (distance symbols in the 20s)
base = bytes(random.randrange(256) for _ in range(20000))
payloads.append(base + base[:8000])
payloads.append(base + bytes(random.randrange(256) for _ in range(200)) + base)

records = []
for p in payloads:
    # Keep the corpus small: the full level x strategy cross-product only for
    # small payloads, a representative subset for the big ones.
    if len(p) > 4096:
        levels = (1, 6, 9)
        strategies = (zlib.Z_DEFAULT_STRATEGY, zlib.Z_FIXED)
    else:
        levels = range(0, 10)
        strategies = (zlib.Z_DEFAULT_STRATEGY, zlib.Z_FIXED, zlib.Z_HUFFMAN_ONLY, zlib.Z_RLE)
    for level in levels:
        for strategy in strategies:
            co = zlib.compressobj(level, zlib.DEFLATED, -15, 9, strategy)
            comp = co.compress(p) + co.flush()
            # sanity: round-trip
            assert zlib.decompress(comp, -15) == p
            records.append((p, comp))

out = bytearray()
out += struct.pack("<I", len(records))
for raw, comp in records:
    out += struct.pack("<II", len(raw), len(comp))
    out += raw
    out += comp

with open(sys.argv[1], "wb") as f:
    f.write(bytes(out))
print(f"{len(records)} records, {len(out)} bytes", file=sys.stderr)
