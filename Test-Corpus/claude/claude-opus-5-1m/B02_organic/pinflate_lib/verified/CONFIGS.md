# CONFIGS.md — configuration-surface table

The mirror of `ERRORS.md`, for **valid** inputs. Rows are derived mechanically
from the branches `c_src/src/lib.c` actually takes, not from what looks
important.

## Public entry points

The library has exactly one function, but its public ABI is larger than that:
all seven exported globals are **writable** (`uint8_t cp_fixed_table[…]`,
not `const`), so re-tuning them is a legitimate runtime configuration that
changes decoding. `nm -D` shows them as `D`/`B`, and a caller can `dlsym` and
overwrite them.

| entry point | kind | reached from |
|---|---|---|
| `pinflate` | function, the only one declared in `include/lib.h` | caller |
| `cp_error_reason` | `const char *`, written by the library, readable/resettable by the caller | `cp_stored`, `cp_block`, `pinflate` |
| `cp_fixed_table` | input table, read by `cp_fixed` | `btype == 1` |
| `cp_permutation_order` | input table, read by `cp_dynamic` | `btype == 2` |
| `cp_len_extra_bits`, `cp_len_base` | input tables, read by `cp_block` | `symbol > 256` |
| `cp_dist_extra_bits`, `cp_dist_base` | input tables, read by `cp_block` | `symbol > 256` |

The internal call hierarchy the rows walk bottom-up:

```
pinflate
├─ cp_read_bits ─ cp_peak_bits / cp_consume_bits / cp_would_overflow
├─ cp_stored    ─ cp_ptr, cp_read_bits
├─ cp_fixed     ─ cp_build(s!=0, 288) + cp_build(0, 32)
├─ cp_dynamic   ─ cp_read_bits, cp_build(0,19), cp_decode, cp_build(s!=0,nlit), cp_build(0,ndst)
└─ cp_block     ─ cp_decode ─ cp_peak_bits, cp_rev16, cp_consume_bits
```

## Axes the C branches on

| axis | values the C distinguishes | C site |
|---|---|---|
| `btype` | 0 stored / 1 fixed / 2 dynamic / 3 reserved | `switch (btype)` :339 |
| `bfinal` | 0 → another block / 1 → return | `do … while (!bfinal)` :371 |
| `(size_t)in & 3` | 0,1,2,3 → `first_bytes` = 0,3,2,1 | :320, pre-load loop :324 |
| `(in_bytes-first_bytes) & 3` | 0 → `final_word_available = 0`; 1,2,3 → `= 1` | :323, :326, :328 |
| `word_count` | `> 0` → `cp_peak_bits` word branch; `== 0` → final-word branch | :100 vs :105 |
| decoded symbol | `< 256` literal / `== 256` end-of-block / `> 256` length | :258, :270, :307 |
| `backwards_distance` | `== 1` → `memset`; else byte loop | `switch` :299 |
| length extra bits | 0 (syms 257–264) / 1–5 (265–284) / 0 (285) | `cp_len_extra_bits` :273 |
| distance extra bits | 0 (dist syms 0–3) / 1–13 (4–29) | `cp_dist_extra_bits` :276 |
| `cp_build` `s` | `!= 0` → fills `s->lookup`; `== 0` → skips | :149, :158 |
| `cp_build` `len` | `<= 9` → `lookup` entry; `> 9` → tree only | :158 |
| `cp_dynamic` symbol | 0–15 direct / 16 copy-prev / 17 short zero run / 18 long zero run | `switch (sym)` :233 |
| `nlit`, `ndst`, `nlen` | 257–288, 1–32, 4–19 | :224–:226 |
| `cp_stored` `LEN` | 0 / 1..n / 65535, plus the 0–7 bit alignment discard | :172, :173, :193 |
| output size | exact / larger / smaller / 1 / 0 | `out_end` :332 |

## Rows

Each row is exercised with **many randomized inputs** (fixed seed, see
`tests/phase_b_valid.rs`), comparing `pinflate`'s return value, the **entire**
output buffer (including the padding past `out_bytes`, so out-of-bounds writes
are caught) and `cp_error_reason`, byte for byte, between the C `.so` and the
Rust `.so` loaded through `libloading`.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C1  | `pinflate` → `cp_stored` → `cp_ptr` | `btype=0`, `bfinal=1`, `LEN=0` (empty stored block) × `in` alignment 0–3 | `c1_stored_empty` | [x] |
| C2  | `pinflate` → `cp_stored` | `btype=0`, `bfinal=1`, `LEN=1..64` random payloads × `in` alignment 0–3 × `out_bytes` exact | `c2_stored_small_random` | [x] |
| C3  | `pinflate` → `cp_stored` | `btype=0`, `bfinal=1`, `LEN` = 65535 (max) | `c3_stored_max_len` | [x] |
| C4  | `pinflate` → `cp_stored` ×2 | two consecutive stored blocks, `bfinal=0` then `1` (multi-block loop) | `c4_stored_two_blocks` | [x] |
| C5  | `pinflate` → `cp_stored` | stored block preceded by a non-byte-aligned header, exercising `cp_read_bits(s, s->count & 7)` with discard 0..7 | `c5_stored_alignment_discard` | [x] |
| C6  | `pinflate` → `cp_fixed` → `cp_build(s!=0,288)`, `cp_build(0,32)` | `btype=1`, literals only, random 1..64 B spanning both fixed code lengths (8-bit symbols 0–143 and 9-bit symbols 144–255) | `c6_fixed_literals_only` | [x] |
| C7  | `pinflate` → `cp_fixed` → `cp_block` | `btype=1`, payload with matches → `symbol > 256`, distance ≠ 1 → byte-copy loop | `c7_fixed_matches_bytecopy` | [x] |
| C8  | `pinflate` → `cp_fixed` → `cp_block` | `btype=1`, run-of-one-byte payload → `backwards_distance == 1` → `memset` path | `c8_fixed_dist1_memset` | [x] |
| C9  | `pinflate` → `cp_fixed` → `cp_block` | `btype=1`, minimum match length 3 and maximum 258 (`cp_len_base[28] == 258`, extra bits 0) | `c9_fixed_length_extremes` | [x] |
| C10 | `pinflate` → `cp_fixed` → `cp_block` | `btype=1`, **every** length symbol 0..28, each at its base, base+1 and base+2^extra-1 (0..5 extra bits) | `c10_fixed_all_length_and_distance_symbols` | [x] |
| C11 | `pinflate` → `cp_dynamic` → `cp_build(0,19)` | `btype=2`, `nlen` at its minimum 4 and maximum 19 | `c11_dynamic_nlen_extremes, c11b_dynamic_nlen_minimum_four` | [x] |
| C12 | `pinflate` → `cp_dynamic` | `btype=2`, code-length symbol 16 (copy previous, 3–6 reps) present | `c12_c13_c14_dynamic_cl_run_symbols` | [x] |
| C13 | `pinflate` → `cp_dynamic` | `btype=2`, code-length symbol 17 (3–10 zeros) present | `c12_c13_c14_dynamic_cl_run_symbols` | [x] |
| C14 | `pinflate` → `cp_dynamic` | `btype=2`, code-length symbol 18 (11–138 zeros) present — sparse alphabet | `c12_c13_c14_dynamic_cl_run_symbols` | [x] |
| C15 | `pinflate` → `cp_dynamic` | `btype=2`, `ndst == 1` (single distance code) — `cp_build(0, dst, …, 1)` | `c15_dynamic_ndst_one` | [x] |
| C16 | `pinflate` → `cp_dynamic` | `btype=2`, `nlit == 288`, `ndst == 32` (both maxima) | `c16_dynamic_nlit_288_ndst_32` | [x] |
| C17 | `pinflate` → `cp_dynamic` → `cp_build` | `btype=2` tree containing codes of length ≤ 9 **and** > 9 (both `lookup` and tree-only paths) | `c17_c35_dynamic_deep_codes` | [x] |
| C18 | `pinflate` → `cp_block` → `cp_decode` | distance symbols with 0 extra bits (distances 1–4) | `c18_c19_distance_symbols` | [x] |
| C19 | `pinflate` → `cp_block` → `cp_decode` | **every** distance symbol 0..29, each at its base, base+1 and base+2^extra-1 (0..13 extra bits), up to distance 32768 | `c18_c19_distance_symbols` | [x] |
| C20 | `pinflate` (multi-block) | payload > 64 KiB across 5 alternating fixed/dynamic blocks, `bfinal=0` loop iterates | `c20_c34_multi_block_mixed_types, c20b_large_payload_multiblock` | [x] |
| C21 | `pinflate` → `cp_peak_bits` | `in_bytes` such that `(in_bytes - first_bytes) & 3 == 0` → `final_word_available == 0` | `c21_c22_c23_final_word_shapes` | [x] |
| C22 | `pinflate` → `cp_peak_bits` | `(in_bytes - first_bytes) & 3 == 1, 2, 3` → `final_word_available == 1`, final-word branch taken | `c21_c22_c23_final_word_shapes` | [x] |
| C23 | `pinflate` → `cp_peak_bits` | `word_count == 0` (whole stream fits in `first_bytes` + final word) | `c21_c22_c23_final_word_shapes` | [x] |
| C24 | `pinflate` | `in` alignment 0,1,2,3 × `out` alignment 0,1,3 — cross-product on a fixed valid stream | `c24_alignment_cross_product` | [x] |
| C25 | `pinflate` | `out_bytes` **exactly** the decompressed size (tight fit, no error) | `c25_c26_out_buffer_sizes` | [x] |
| C26 | `pinflate` | `out_bytes` larger than needed (padding must stay untouched) | `c25_c26_out_buffer_sizes` | [x] |
| C27 | `pinflate` | all three block types × payload shapes: empty, 1 B, many literals, 4-symbol low-entropy, 256-symbol with far matches, and pure RLE runs; × `ClMode::Raw`/`Rle` | `c27_c37_randomized_sweep` | [x] |
| C28 | `pinflate` → `cp_fixed` | caller **overwrites `cp_fixed_table`** with a different valid full code-length set before the call (mutable-global configuration axis) | `c28_caller_rewrites_fixed_table` | [x] |
| C29 | `pinflate` → `cp_dynamic` | caller **permutes `cp_permutation_order`** and feeds a header written in that permutation | `c29_caller_permutes_permutation_order` | [x] |
| C30 | `pinflate` → `cp_block` | caller **rewrites `cp_len_base` / `cp_len_extra_bits`** (e.g. all-zero extra bits) and decodes a fixed-block stream | `c30_c31_caller_rewrites_length_and_distance_tables` | [x] |
| C31 | `pinflate` → `cp_block` | caller **rewrites `cp_dist_base` / `cp_dist_extra_bits`** and decodes a fixed-block stream | `c30_c31_caller_rewrites_length_and_distance_tables` | [x] |
| C32 | `cp_error_reason` | caller pre-sets it to a sentinel; a **successful** `pinflate` must leave it untouched (the C never clears it) | `c32_error_reason_untouched_on_success` | [x] |
| C33 | `pinflate` | `bfinal=1` on the very first block for each of `btype` 0/1/2 (single-block streams) | `c33_single_block_each_btype` | [x] |
| C34 | `pinflate` | mixed block types in one stream: stored → fixed → dynamic → final | `c20_c34_multi_block_mixed_types` | [x] |
| C35 | `pinflate` → `cp_decode` | tree whose maximum code length is 15 (deepest legal Huffman code) | `c17_c35_dynamic_deep_codes` | [x] |
| C36 | `pinflate` → `cp_build` | literal/length tree where symbol 256 (end-of-block) has the shortest code, and where it has a long code | `c36_end_of_block_code_lengths` | [x] |
| C37 | `pinflate` | randomized valid streams: 512 random payloads × random level/strategy × random `in`/`out` alignment (property-style sweep over C1–C36's cross-product) | `c27_c37_randomized_sweep` | [x] |
| C38 | `pinflate` | trailing garbage after the `bfinal` block (must be ignored — the loop exits) | `c38_c39_trailing_garbage` | [x] |
| C39 | `pinflate` | `in_bytes` larger than the actual stream (extra padding bytes present in the buffer) | `c38_c39_trailing_garbage` | [x] |
| C40 | `pinflate` | 65 536 bytes produced entirely by length-258 `backwards_distance == 1` matches → maximal `memset` runs | `c40_large_rle_memset_runs` | [x] |

## How the streams are built

Rows are driven by a small DEFLATE **encoder** written for the tests
(`tests/common/enc.rs`), not by compressing with a third-party library. A
compressor only emits the streams it happens to like, whereas these rows name
specific C branches — `backwards_distance == 1`, code-length symbol 18,
`ndst == 1`, `nlen == 4`, code lengths above 9, length symbol 287 — and the
encoder can guarantee each one is taken. The encoder asserts its own
invariants (Kraft sum exactly 1, code-length code lengths ≤ 7 so they fit the
header's 3-bit field, literal/distance lengths ≤ 15), and every row additionally
asserts that the C library really does decode the stream it was handed, so a row
cannot silently degrade into an error-path test.

Two rows are exceptions and say so inline: the C's stored-block path is only
correct for some input sizes (`cp_stored`'s length check is inverted and
`cp_ptr` derives the payload address from a `word_index` that the final-word
refill never advances), so C2–C5, C20 and C34 assert *agreement* plus "at least
one variant took the success path" via `assert_some_decode`, rather than
asserting the C decodes correctly.

## Completion

* 40 rows, all [x], across 31 tests in `tests/phase_b_valid.rs`.
* Each row runs many randomized inputs from a fixed seed (`Rng`, xorshift64*),
  and compares `pinflate`'s return value, `cp_error_reason`, and the **entire**
  output buffer including padding past `out_bytes`.
* Additional coverage outside the row table: `verify/validonly.py` cross-checks
  3 900 real zlib-produced streams (levels 0/1/2/6/9 × 5 strategies × 13
  payloads up to 70 000 B × 4 input alignments × 3 output alignments), and
  `verify/probe_fork.py` cross-checks 2 500 fuzzed/mutated/truncated streams —
  0 divergences.
