# CONFIGS.md — configuration surface for VALID inputs (Phase A)

## Public entry points

`c_src/include/lib.h` exports exactly one function, and `nm -D` exports seven
mutable data objects. There are no `static`-free helpers, so **the complete
public surface is**:

| entry point | kind | reached code |
|-------------|------|--------------|
| `pinflate(in, in_bytes, out, out_bytes)` | function | everything |
| `cp_fixed_table[288+32]` | writable table | `cp_fixed` → `cp_build` (both trees) |
| `cp_permutation_order[19]` | writable table | `cp_dynamic` code-length permutation |
| `cp_len_extra_bits[31]`, `cp_len_base[31]` | writable tables | `cp_block` match length |
| `cp_dist_extra_bits[32]`, `cp_dist_base[32]` | writable tables | `cp_block` match distance |
| `cp_error_reason` | writable pointer | all 6 error paths |

The 14 internal functions (`cp_would_overflow`, `cp_ptr`, `cp_peak_bits`,
`cp_consume_bits`, `cp_read_bits`, `cp_rev16`, `cp_build`, `cp_stored`,
`cp_fixed`, `cp_decode`, `cp_dynamic`, `cp_block`, `cp_make_pixel`,
`cp_make_pixel_a`) have internal linkage and cannot be called across the FFI
boundary, so "lowest level first" here means: drive each of them directly through
the smallest bit-stream that reaches it, and through the writable tables it reads
— which is what the rows below do (rows 1–16 are pure bit-reader / `cp_ptr` /
`cp_peak_bits` coverage, 17–24 `cp_build`+`cp_decode` via the fixed tree, 25–40
`cp_dynamic`, 41–52 `cp_block` copy paths, 53–60 the writable tables, 61–66
multi-block composition).

## Axes the C actually branches on

1. `in` alignment ⇒ `first_bytes = ((in+3) & ~3) - in ∈ {0,1,2,3}` (seeds `bits`
   byte-wise, sets `count = first_bytes*8`, moves `words`).
2. `(in_bytes - first_bytes) & 3 ⇒ last_bytes ∈ {0,1,2,3}` ⇒
   `final_word_available` and the `count += bits_left` fold in `cp_peak_bits`.
   Note `word_count = (in_bytes-first_bytes)/4` truncates, so
   `first_bytes + 4*word_count + last_bytes == in_bytes` always, but the *fold*
   path (`else if (s->final_word_available)`) only exists when `last_bytes != 0`.
3. `btype ∈ {0,1,2}` (3 is an error, `ERRORS.md` E6).
4. `bfinal`: 1 block vs. N blocks, and every ordered pair of block types.
5. `cp_stored`: `LEN ∈ {0, 1, 2, 3, …, 65535}`, `bits_left/8 == LEN` (the only
   accepted relation) — and the `count & 7` pre-read.
6. Literal values (`char` is signed on x86-64: `0x00`, `0x7F`, `0x80`, `0xFF`).
7. Length symbols `257…285` (all 29 `cp_len_extra_bits`/`cp_len_base` rows) plus
   the two "impossible" fixed-tree symbols `286`, `287` ⇒ length 0.
8. Distance symbols `0…29` (all 30 rows) plus `30`, `31` ⇒ distance 0.
9. `switch (backwards_distance) case 1:` (`memset`) vs `default:` (byte loop),
   and `distance < length` (overlapping / RLE) vs `distance >= length`.
10. `out_bytes`: exactly the decompressed size, larger, `INT_MAX`.
11. `cp_dynamic`: `HLIT+257 ∈ 257…288`, `HDIST+1 ∈ 1…32`, `HCLEN+4 ∈ 4…19`.
12. `cp_dynamic` code-length alphabet: direct `0…15`, `16` (copy previous 3–6),
    `17` (zeros 3–10), `18` (zeros 11–138).
13. `cp_build`: `len <= 9` (writes the 512-entry `lookup`, `j += 1<<len` loop) vs
    `len ∈ 10…15` (no `lookup` write); `s == NULL` (dist tree, no `lookup`
    memset) vs `s != NULL` (lit tree).
14. `cp_decode` binary search shapes: 1-symbol tree, 2-symbol tree, full
    288-symbol tree, and trees whose `first[15]` (`max_index`) is < the symbol
    count.
15. The 7 writable tables: unmodified vs. modified through the `.so` export.
16. Empty output (`EOB` as the first symbol), 1-byte output, ≥64 KiB output.

## Rows (each row = one differential test configuration, randomized inputs)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|------------------------------------------|-----|
| 1 | `pinflate` | `first_bytes=0`, `last_bytes=0`, single fixed block, random literals | [x] |
| 2 | `pinflate` | `first_bytes=0`, `last_bytes=1` | [x] |
| 3 | `pinflate` | `first_bytes=0`, `last_bytes=2` | [x] |
| 4 | `pinflate` | `first_bytes=0`, `last_bytes=3` | [x] |
| 5 | `pinflate` | `first_bytes=1`, `last_bytes=0` | [x] |
| 6 | `pinflate` | `first_bytes=1`, `last_bytes=1` | [x] |
| 7 | `pinflate` | `first_bytes=1`, `last_bytes=2` | [x] |
| 8 | `pinflate` | `first_bytes=1`, `last_bytes=3` | [x] |
| 9 | `pinflate` | `first_bytes=2`, `last_bytes=0` | [x] |
| 10 | `pinflate` | `first_bytes=2`, `last_bytes=1` | [x] |
| 11 | `pinflate` | `first_bytes=2`, `last_bytes=2` | [x] |
| 12 | `pinflate` | `first_bytes=2`, `last_bytes=3` | [x] |
| 13 | `pinflate` | `first_bytes=3`, `last_bytes=0` | [x] |
| 14 | `pinflate` | `first_bytes=3`, `last_bytes=1` | [x] |
| 15 | `pinflate` | `first_bytes=3`, `last_bytes=2` | [x] |
| 16 | `pinflate` | `first_bytes=3`, `last_bytes=3` | [x] |
| 17 | `pinflate` (`cp_stored`) | one stored block, `LEN = 0`, `in_bytes` chosen so `bits_left/8 == LEN` | [x] |
| 18 | `pinflate` (`cp_stored`) | one stored block, `LEN = 1` | [x] |
| 19 | `pinflate` (`cp_stored`) | one stored block, `LEN ∈ 2…4096` random, all 4 `in` alignments | [x] |
| 20 | `pinflate` (`cp_stored`) | one stored block, `LEN = 65535` (max), `out_bytes == LEN` | [x] |
| 21 | `pinflate` (`cp_stored`) | stored block, `out_bytes < LEN` (no `out_end` check in C ⇒ deliberate overflow into the padded out buffer) | [x] |
| 22 | `pinflate` (`cp_fixed`/`cp_block`) | fixed block, EOB only (empty output), `out_bytes = 0` | [x] |
| 23 | `pinflate` (`cp_fixed`/`cp_block`) | fixed block, literals only: all 256 byte values, plus random | [x] |
| 24 | `pinflate` (`cp_fixed`/`cp_block`) | fixed block, literal symbols including the 9-bit range (144…255) and 8-bit range | [x] |
| 25 | `pinflate` (`cp_block`) | fixed block, every length symbol `257…285` × min/mid/max extra-bit value, distance symbol 0 (`dist=1`, `memset` path) | [x] |
| 26 | `pinflate` (`cp_block`) | fixed block, every distance symbol `0…29` × min/mid/max extra-bit value, length 3 | [x] |
| 27 | `pinflate` (`cp_block`) | fixed block, length symbols `286`, `287` ⇒ `cp_len_base == 0` ⇒ length 0, followed by a distance symbol (0-byte copy) | [x] |
| 28 | `pinflate` (`cp_block`) | fixed block, distance symbols `30`, `31` ⇒ `cp_dist_base == 0` ⇒ `backwards_distance == 0`, `src == dst` (no-op copy of `length` bytes) | [x] |
| 29 | `pinflate` (`cp_block`) | fixed block, `backwards_distance == 1` (`memset` path) with length 3…258 | [x] |
| 30 | `pinflate` (`cp_block`) | fixed block, overlapping copy `1 < distance < length` (byte loop, forward propagation) | [x] |
| 31 | `pinflate` (`cp_block`) | fixed block, `distance == length`, and `distance > length` | [x] |
| 32 | `pinflate` (`cp_block`) | fixed block, `distance` exactly equal to the bytes emitted so far (`out - distance == begin`, boundary of E4) | [x] |
| 33 | `pinflate` (`cp_block`) | fixed block, match ending exactly at `out_end` (boundary of E5) | [x] |
| 34 | `pinflate` (`cp_block`) | fixed block, ≥64 KiB output built from long matches (distance up to 32768) | [x] |
| 35 | `pinflate` (`cp_dynamic`) | dynamic block, `HLIT=257`, `HDIST=1`, `HCLEN=19`, direct code lengths only | [x] |
| 36 | `pinflate` (`cp_dynamic`) | dynamic block, `HLIT=288`, `HDIST=32`, `HCLEN=19`, random complete trees | [x] |
| 37 | `pinflate` (`cp_dynamic`) | dynamic block, `HCLEN` swept `5…19` (each value that can still describe the tree) | [x] |
| 38 | `pinflate` (`cp_dynamic`) | dynamic block using code-length symbol `16` (repeat previous, extra 0…3) | [x] |
| 39 | `pinflate` (`cp_dynamic`) | dynamic block using code-length symbol `17` (3…10 zeros) | [x] |
| 40 | `pinflate` (`cp_dynamic`) | dynamic block using code-length symbol `18` (11…138 zeros) | [x] |
| 41 | `pinflate` (`cp_dynamic`) | dynamic block using 16/17/18 mixed, `n` landing exactly on `nlit+ndst` | [x] |
| 42 | `pinflate` (`cp_dynamic`) | dynamic block with all code lengths ≤ 9 (⇒ `cp_build` fills `lookup`) | [x] |
| 43 | `pinflate` (`cp_dynamic`) | dynamic block with code lengths in 10…15 (⇒ `cp_build` skips `lookup`) | [x] |
| 44 | `pinflate` (`cp_dynamic`) | dynamic block, literal tree = single symbol 256 with length 1 (under-subscribed, `max_index == 1`) | [x] |
| 45 | `pinflate` (`cp_dynamic`) | dynamic block, distance tree with exactly 1 symbol (`HDIST=1`, length 1) and matches | [x] |
| 46 | `pinflate` (`cp_dynamic`) | dynamic block, distance tree with 2 symbols, lengths {1,1} | [x] |
| 47 | `pinflate` (`cp_dynamic`) | dynamic block, `cp_permutation_order` fully exercised: all 19 code-length slots non-zero | [x] |
| 48 | `pinflate` (`cp_dynamic`) | dynamic block, random complete trees × all 4 `in` alignments × all 4 `last_bytes` | [x] |
| 49 | `pinflate` (`cp_dynamic`) | dynamic block, matches using all length/distance symbols reachable from the random tree | [x] |
| 50 | `pinflate` | `out_bytes` exactly the decompressed size (all block types) | [x] |
| 51 | `pinflate` | `out_bytes` larger than needed, and `out_bytes = INT_MAX` | [x] |
| 52 | `pinflate` | `out` unaligned (offsets 0…7 into the padded buffer) | [x] |
| 53 | `cp_fixed_table` + `pinflate` | rotate the fixed literal/distance code lengths into another *valid* complete pair of trees, then decode a fixed block | [x] |
| 54 | `cp_fixed_table` + `pinflate` | swap the 7-bit and 8-bit groups (still a complete tree) | [x] |
| 55 | `cp_len_base` + `pinflate` | change `cp_len_base[0]` (length of symbol 257) to another value ⇒ different copy length | [x] |
| 56 | `cp_len_extra_bits` + `pinflate` | change `cp_len_extra_bits[0]` from 0 to 3 ⇒ 3 extra bits consumed for symbol 257 | [x] |
| 57 | `cp_dist_base` + `pinflate` | change `cp_dist_base[0]` from 1 to 4 ⇒ different backwards distance | [x] |
| 58 | `cp_dist_extra_bits` + `pinflate` | change `cp_dist_extra_bits[0]` from 0 to 5 | [x] |
| 59 | `cp_permutation_order` + `pinflate` | reverse the permutation table and encode a dynamic block accordingly | [x] |
| 60 | `cp_error_reason` + `pinflate` | pre-set to a non-null value, then run a *successful* stream ⇒ must stay untouched | [x] |
| 61 | `pinflate` | 2 blocks: stored → fixed (`bfinal` on the 2nd) | [x] |
| 62 | `pinflate` | 2 blocks: fixed → stored | [x] |
| 63 | `pinflate` | 2 blocks: dynamic → fixed, matches crossing the block boundary (`begin` is the whole out buffer, not the block start) | [x] |
| 64 | `pinflate` | 2 blocks: fixed → dynamic, second block re-builds `lit`/`dst`/`lookup` | [x] |
| 65 | `pinflate` | 3 blocks: stored → dynamic → stored | [x] |
| 66 | `pinflate` | N∈{1..6} random blocks of random types, random data, × alignments (property test, 512 seeds) | [x] |
| 67 | `pinflate` (`cp_stored`) | stored block whose header runs into the final partial word (`last_bytes != 0`), so `cp_peak_bits` folds `final_word` and `cp_ptr`'s `words + word_index - count/8` no longer matches the real byte position — the C copies from the *wrong* offset and the translation must copy from the same wrong offset | [x] |
| 68 | `pinflate` | trailing input bytes after the final block (`bfinal == 1`), i.e. `in_bytes` larger than the stream, at every alignment | [x] |
| 69 | `pinflate` | `in_bytes` *smaller* than the buffer (truncation at every offset of a valid stream) | [x] |

| 70 | `pinflate` | several calls in a row on the same input/output buffers (fixed / stored / dynamic streams and an error stream), verifying that nothing leaks between calls and that `cp_error_reason` keeps its value | [x] |

## Where each row is verified

| rows | test id in `tests/differential.rs` | cases |
|------|------------------------------------|-------|
| 1–16, 68 | `b_align` | 640 |
| 17–20 | `b_stored` | 24 |
| 19, 67 | `b_stored_folded` | 48 |
| 21 | `b_stored_overflow` | 24 |
| 22–24 | `b_fixed_lit` | 72 |
| 25, 27, 29 | `b_len_syms` (every length symbol 257…287 × min/mid/max extra bits) | 67 |
| 26, 28 | `b_dist_syms` (every distance symbol 0…31 × min/mid/max extra bits) | 82 |
| 30–33 | `b_copy_paths` | 266 |
| 34 | `b_big` (≥ 70 000-byte outputs, distances up to 32768) | 3 |
| 35–37 | `b_dyn_basic` | 32 |
| 38–41 | `b_dyn_rle` (all 7 combinations of the repeat codes 16/17/18) | 112 |
| 42–46 | `b_dyn_lens` | 28 |
| 47–49 | `b_dyn_random` | 512 |
| 50–52 | `b_out_sizes` | 128 |
| 53–59 | `b_tables` (all six writable tables) | 13 |
| 60 | `b_reason` | 4 |
| 61–65 | `b_multi` | 44 |
| 66 | `b_property` (random block sequences × alignments × out sizes) | 1536 |
| 69 | `c_boundaries` | 45 |
| 70 | `b_repeat_calls` | 96 |

Every row is compared byte-for-byte between the two `.so`s: return value,
`cp_error_reason`, the **whole padded output allocation** (so an out-of-range
write is caught even when the declared `out_bytes` region matches), and the six
exported tables. Where the expected output is computable from the encoder it is
additionally asserted against the C (`Expect::Out`), so a test cannot silently
degenerate into "both libraries do nothing".

Total: **16 082 cases, 0 failures**, against both the `debug` and the `release`
`cdylib`.
