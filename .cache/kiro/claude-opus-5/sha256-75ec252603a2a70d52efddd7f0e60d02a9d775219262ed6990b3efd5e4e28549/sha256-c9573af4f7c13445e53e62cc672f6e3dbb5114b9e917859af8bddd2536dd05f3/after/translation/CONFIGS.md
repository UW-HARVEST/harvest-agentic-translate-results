# CONFIGS.md — configuration-surface table

Axes derived mechanically from the branches in `c_src/src/lib.c` and the public
surface in `c_src/include/lib.h` + `nm -D`.

## Public entry points

| entry point | kind |
|---|---|
| `load_png_mem(const uint8_t*, int)` | high-level one-shot wrapper |
| `cp_inflate(void*, int, void*, int)` | **low-level** raw-DEFLATE entry point (exported, not in `lib.h`) |
| `cp_fixed_table[320]` | exported **writable** table — drives `cp_build` for `btype==1` |
| `cp_permutation_order[19]` | exported writable table — drives `cp_dynamic`'s code-length order |
| `cp_len_extra_bits[31]`, `cp_len_base[31]` | exported writable tables — drive length decoding in `cp_block` |
| `cp_dist_extra_bits[32]`, `cp_dist_base[32]` | exported writable tables — drive distance decoding in `cp_block` |
| `cp_error_reason` | exported readable/writable `const char *` |

## Branch axes found in the C

* `load_png_mem`: `switch (color_type)` → `0,2,3,4,6` ⇒ `bpp = 1,3,1,2,4`;
  `if (color_type == 3)` → `cp_depalette` else `cp_convert`;
  `cp_find("PLTE")` present/absent; `cp_find("tRNS")` present/absent;
  IDAT loop = `cp_find` once then `cp_chunk` repeatedly ⇒ 1 vs N *consecutive* IDATs.
* `cp_convert`: `switch (bpp)` → `1,2,3,4`.
* `cp_get_alpha_for_indexed_image`: `!trns` / `index >= trns_len` / `index < trns_len`.
* `cp_unfilter`: `switch (*raw++)` → `0,1,2,3,4` for the **first** row (a distinct,
  reduced code path: `b`/`c` are forced to 0 and `x` starts at `bpp`) and again
  for **subsequent** rows (full Sub/Up/Average/Paeth with `prev`).
* `cp_inflate`: `switch (btype)` → `0` stored / `1` fixed / `2` dynamic;
  `do { … } while (!bfinal)` ⇒ 1 vs N blocks; `first_bytes` = `in` alignment 0–3;
  `last_bytes` = `(in_bytes-first_bytes) & 3` ⇒ 0–3 (`final_word_available`).
* `cp_block`: `symbol < 256` / `== 256` / `> 256`;
  `switch (backwards_distance) { case 1: memset; default: byte copy }`.
* `cp_peak_bits`: `word_index < word_count` / `final_word_available` / neither.
* `cp_dynamic`: `switch (sym)` → `16` (copy previous 3–6), `17` (0 × 3–10),
  `18` (0 × 11–138), `default` (literal length).
* `cp_build`: `s != NULL` (builds the 512-entry `lookup`) vs `s == NULL`; `len <= 9` vs `> 9`.

## Configuration rows

Every row is driven with **many** randomized inputs (fixed seed 0x5EED_C0DE,
`tests/common/mod.rs::Rng`), comparing C vs Rust byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `cp_inflate` | `btype=0` stored block, `bfinal=1`, LEN ∈ {1..64} random, output exactly LEN | [x] |
| 2 | `cp_inflate` | `btype=0` stored, LEN = 0 (empty stored block) | [x] |
| 3 | `cp_inflate` | `btype=1` fixed Huffman, literals only, all 256 byte values, lengths 1..300 | [x] |
| 4 | `cp_inflate` | `btype=1` fixed Huffman, literal+length/distance pairs, **distance == 1** (`memset` path) | [x] |
| 5 | `cp_inflate` | `btype=1` fixed Huffman, length/distance pairs, distance > 1, overlapping copy (`dist < len`) | [x] |
| 6 | `cp_inflate` | `btype=1` fixed Huffman, sweep **all 29 length codes** (`cp_len_base[0..29]`, len 3..258) | [x] |
| 7 | `cp_inflate` | `btype=1` fixed Huffman, sweep **all 30 distance codes** (`cp_dist_base[0..30]`, dist 1..32768) | [x] |
| 8 | `cp_inflate` | `btype=2` dynamic Huffman (zlib/miniz-produced), random data, code lengths ≤ 9 (lookup-table path) | [x] |
| 9 | `cp_inflate` | `btype=2` dynamic Huffman with code lengths > 9 (skewed symbol distribution ⇒ deep tree, binary-search path only) | [x] |
| 10 | `cp_inflate` | `btype=2` dynamic Huffman exercising RLE code-length symbols 16/17/18 (long zero runs + repeats in the code-length alphabet) | [x] |
| 11 | `cp_inflate` | **multi-block**: `bfinal=0` blocks followed by a final one, mixing stored/fixed/dynamic | [x] |
| 12 | `cp_inflate` | `in` pointer alignment ∈ {0,1,2,3} (⇒ `first_bytes` 0..3) × `in_bytes` ≡ {0,1,2,3} mod 4 (⇒ `last_bytes`, `final_word_available`) | [x] |
| 13 | `cp_inflate` | `out_bytes` exactly the decompressed size vs. larger than needed | [x] |
| 14 | `load_png_mem` | `color_type=0` (grey, `bpp=1`), filter 0 on every row, random dims 1..17 × 1..17 | [x] |
| 15 | `load_png_mem` | `color_type=0`, filter type **1 (Sub)** on every row | [x] |
| 16 | `load_png_mem` | `color_type=0`, filter type **2 (Up)** on every row | [x] |
| 17 | `load_png_mem` | `color_type=0`, filter type **3 (Average)** on every row | [x] |
| 18 | `load_png_mem` | `color_type=0`, filter type **4 (Paeth)** on every row | [x] |
| 19 | `load_png_mem` | `color_type=0`, **random filter type per row** (0..4), incl. row 0 taking each of the 5 reduced first-row paths | [x] |
| 20 | `load_png_mem` | `color_type=2` (RGB, `bpp=3`), random filter per row | [x] |
| 21 | `load_png_mem` | `color_type=4` (grey+alpha, `bpp=2`), random filter per row | [x] |
| 22 | `load_png_mem` | `color_type=6` (RGBA, `bpp=4`), random filter per row | [x] |
| 23 | `load_png_mem` | `color_type=3` (indexed, `bpp=1`) + PLTE, **no tRNS** (`cp_get_alpha_for_indexed_image` → 255) | [x] |
| 24 | `load_png_mem` | `color_type=3` + PLTE + tRNS with `trns_len == palette entries` (all indices < `trns_len`) | [x] |
| 25 | `load_png_mem` | `color_type=3` + PLTE + tRNS with `trns_len < palette entries` (mixes both branches of `cp_get_alpha_for_indexed_image`) | [x] |
| 26 | `load_png_mem` | `color_type=3` + PLTE + tRNS where indices exceed the 256-entry palette is impossible, but PLTE **shorter than 256 entries** ⇒ `plte[c*3]` reads past the chunk (must match) | [x] |
| 27 | `load_png_mem` | 1×1 image, every colour type (minimal shape) | [x] |
| 28 | `load_png_mem` | w=1, h=many (tall) and w=many, h=1 (wide), every colour type | [x] |
| 29 | `load_png_mem` | IDAT split across **2** consecutive chunks | [x] |
| 30 | `load_png_mem` | IDAT split across **many (5+)** consecutive chunks, random split points | [x] |
| 31 | `load_png_mem` | ancillary chunks (`gAMA`, `pHYs`, `tEXt`) interleaved before PLTE / between PLTE and IDAT | [x] |
| 32 | `load_png_mem` | chunk order PLTE → tRNS → IDAT vs tRNS → PLTE → IDAT (the C does two sequential `cp_find`s, so order changes what is found) | [x] |
| 33 | `load_png_mem` | zlib header CINFO sweep: `data[0] & 0xf0` ∈ {0x00,0x10,…,0x70} with CM=8 | [x] |
| 34 | `load_png_mem` | zlib FLG `FCHECK`/`FLEVEL` bits varied (`data[1] & ~0x20`) — not validated by the C, must still decode | [x] |
| 35 | `load_png_mem` | IDAT payload produced with **stored** deflate blocks (`btype=0`) instead of compressed | [x] |
| 36 | `load_png_mem` | IDAT payload produced with **fixed** Huffman (`btype=1`) | [x] |
| 37 | `load_png_mem` | IDAT payload produced with **dynamic** Huffman (`btype=2`), incl. long back-references across scanlines | [x] |
| 38 | `load_png_mem` | trailing chunks after the last IDAT (`IEND`, plus junk) | [x] |
| 39 | `load_png_mem` | larger image (64×64) each colour type, random pixel data, random filters — exercises `cp_block`'s copy loop and `cp_build`'s `len > 9` path | [x] |
| 40 | tables + `cp_inflate` | `cp_fixed_table` mutated (all 8s → uniform 8-bit code lengths) before a `btype=1` block | [x] |
| 41 | tables + `cp_inflate` | `cp_permutation_order` mutated before a `btype=2` block | [x] |
| 42 | tables + `cp_inflate` | `cp_len_base` / `cp_len_extra_bits` mutated before a `btype=1` block with length codes | [x] |
| 43 | tables + `cp_inflate` | `cp_dist_base` / `cp_dist_extra_bits` mutated before a `btype=1` block with distance codes | [x] |
| 44 | tables (read) | contents of all 6 exported tables compared byte-for-byte via `dlsym` | [x] |
| 45 | `load_png_mem` | `cp_error_reason` left over from a previous successful call is **not** cleared (state carry-over between calls) | [x] |
| 46 | `cp_inflate` | `out_bytes` large, input decodes to fewer bytes — trailing output bytes untouched (verified deterministic via double-run) | [x] |
| 47 | `load_png_mem` | image where `(w+1)*h*bpp` differs from `(w*bpp+1)*h` (`bpp>1`) so the `out` offset trick is exercised for every `bpp` | [x] |
| 48 | `load_png_mem` | filter byte sweep 0..=255 on row 0 and on row 1 (valid 0..4 and invalid ≥5 in the same sweep) | [x] |

---

## Phase B results — every row passes across randomized inputs

Tests in `tests/phase_b_valid.rs`, one per row, seed `0x5EED_C0DE_5EED_C0DE`
(`tests/common/mod.rs::SEED`). Row *n* is `rowNN_*`. Streams are built by
hand-rolled DEFLATE writers in `tests/common/deflate.rs` (stored / fixed /
dynamic, an LZ77 tokenizer, and a length-limited Huffman builder) so each axis is
directly controllable; `flate2` is used in row 37b as an independent
cross-check. `tests/common/png.rs` also contains an independent reference model
of `cp_unfilter` + `cp_convert` + `cp_depalette`, so most rows assert not only
"C == Rust" but also "C == model".

| # | test | randomized inputs | [x] |
|---|------|-------------------|-----|
| 1 | `row01_inflate_stored_random_len` | 200 random lengths + 7 fixed (up to 65535) + 160 across 4 `in` alignments | [x] |
| 2 | `row02_inflate_stored_empty` | 4 alignments × 3 output sizes | [x] |
| 3 | `row03_inflate_fixed_literals` | 8 boundary lengths (143/144/145/256/300) + 150 random | [x] |
| 4 | `row04_inflate_fixed_distance_one_memset` | 120 random run lengths 3..258 | [x] |
| 5 | `row05_inflate_fixed_overlapping_copy` | 200 random (prefix, dist, len) with `dist < len` | [x] |
| 6 | `row06_inflate_all_length_codes` | all 29 length codes × low/mid/high extra-bit values | [x] |
| 7 | `row07_inflate_all_distance_codes` | all 30 distance codes × low/mid/high, up to `dist = 32768` | [x] |
| 8 | `row08_inflate_dynamic_shallow` | 80 streams, depth ≤ 9 (`lookup` table path) | [x] |
| 9 | `row09_inflate_dynamic_deep` | 40 streams, asserted `max code length > 9` (binary-search path) | [x] |
| 10 | `row10_inflate_dynamic_rle_code_lengths` | 2 RLE modes × 6 (HLIT, HDIST) shapes × 12 streams = 144 | [x] |
| 11 | `row11_inflate_multi_block` | 60 random 2–5 block chains mixing fixed/dynamic + 30 fixed→stored | [x] |
| 12 | `row12_inflate_alignment_matrix` | 4 alignments × 4 length residues × 20 = 320 | [x] |
| 13 | `row13_inflate_out_bytes_slack` | 120 lengths × 5 slack sizes; also asserts the untouched tail keeps the caller's `0xAA` | [x] |
| 14–18 | `row14`…`row18` | filter 0/1/2/3/4 on every row, 25 random geometries each | [x] |
| 19 | `row19_grey_random_filters_and_first_row_paths` | 25 random filter vectors + all 5 first-row paths × 3 heights | [x] |
| 20 | `row20_rgb_random_filters` | random + each fixed filter, 25 geometries each (150) | [x] |
| 21 | `row21_greyalpha_random_filters` | same (150) | [x] |
| 22 | `row22_rgba_random_filters` | same (150) | [x] |
| 23 | `row23_indexed_no_trns` | 30 random geometries | [x] |
| 24 | `row24_indexed_full_trns` | 30 random geometries, `trns_len == 256` | [x] |
| 25 | `row25_indexed_short_trns` | `trns_len ∈ {0,1,2,17,128,255}` × 8 | [x] |
| 26 | `row26_indexed_short_plte` | PLTE of 1/2/16/100/255 entries × 8 (C reads past the chunk; differential only) | [x] |
| 27 | `row27_one_by_one_every_color_type` | 5 colour types × 3 block types | [x] |
| 28 | `row28_tall_and_wide` | 5 colour types × 7 shapes incl. 1×200 and 200×1 | [x] |
| 29 | `row29_idat_two_chunks` | 5 colour types × 6 | [x] |
| 30 | `row30_idat_many_chunks` | 3/5/8/13 IDATs × 5 colour types | [x] |
| 31 | `row31_ancillary_chunks` | gAMA/cHRM before, pHYs/tEXt/bKGD between, × 5 × 6 | [x] |
| 32 | `row32_plte_trns_order` | both orders × 12 (model checked only for PLTE-first) | [x] |
| 33 | `row33_zlib_cinfo_sweep` | all 8 valid CINFO values | [x] |
| 34 | `row34_zlib_flg_sweep` | 8 FLG values with FDICT clear | [x] |
| 35 | `row35_png_stored_blocks` | 5 colour types × 8 | [x] |
| 36 | `row36_png_fixed_blocks` | literals-only and LZ77 × 5 × 6 | [x] |
| 37 | `row37_png_dynamic_blocks` | 4 dynamic configurations × 5 × 5 | [x] |
| 37b | `row37b_png_flate2_streams` | independent compressor, 3 levels × 5 colour types | [x] |
| 38 | `row38_trailing_chunks` | tEXt/zTXt after the last IDAT × 5 × 6 | [x] |
| 39 | `row39_large_images` | 64×64 all colour types × 2 encodings, plus a run-heavy 64×40 image | [x] |
| 40 | `row40_mutated_cp_fixed_table` | 12 random-but-complete 288+32 code-length tables written into both `.so`s | [x] |
| 41 | `row41_mutated_cp_permutation_order` | 12 random permutations of 0..18 | [x] |
| 42 | `row42_mutated_length_tables` | 20 random `cp_len_base` / `cp_len_extra_bits` states | [x] |
| 43 | `row43_mutated_distance_tables` | 20 random `cp_dist_base` / `cp_dist_extra_bits` states | [x] |
| 44 | `phase_d_parity::exported_table_contents_match` | all 6 tables compared byte-for-byte via `dlsym` | [x] |
| 45 | `row45_error_reason_carry_over` | failure then success in one child; the stale reason must survive | [x] |
| 46 | `row13_inflate_out_bytes_slack` | asserted explicitly (tail stays `0xAA`) | [x] |
| 47 | `row47_out_offset_trick_every_bpp` | 5 colour types × 6 shapes | [x] |
| 48 | `row48_filter_byte_full_sweep` | filter byte 0..=255 on row 0 and row 1 × 3 colour types = 1536 | [x] |

**Result: 47/47 tests pass** under both the release and the debug Rust `.so`.

## Beyond the enumerated rows

`tests/fuzz_diff.rs` adds ~6000 randomized differential cases that nobody
enumerated: bit-flipped valid PNGs (1200), truncations (600), random bytes as
PNG (400), random IHDR field combinations (600), random chunk streams with
overflowing/sign-extending declared lengths (500), random deflate streams
(1500), mutated valid deflate streams (800), random exported-table states (300)
and random filter/palette shapes (600). **9/9 pass.** Coverage observed:

* bit-flipped PNGs: 696 decoded, 490 rejected, 14 died by signal;
* random deflate: 18 decoded, 954 rejected, 528 died by signal.

## Findings worth recording

Two behaviours of the C that the tests had to be written *around*, because the
first drafts asserted the wrong thing:

1. **`cp_stored` copies from the wrong offset at most `in` alignments.**
   `cp_ptr` computes the memcpy source as
   `(char *)(s->words + s->word_index) - (s->count / 8)`, which only lands on
   the payload when `in` is 2 bytes past a 4-byte boundary — i.e. exactly what
   `load_png_mem` passes (`data + 2` off a 16-aligned `malloc`). Calling
   `cp_inflate` directly with a 4-aligned buffer makes a stored block copy the
   LEN/NLEN bytes instead of the data. Row 1 therefore model-checks only the
   `in_shift == 2` case and compares the other three alignments
   differentially. The Rust reproduces all four.
2. **The pixel buffer's tail is uninitialised by design.** `img.pix` is
   `malloc((img.w+1)*img.h*4)` but `cp_convert` only writes `img.w*img.h*4`
   bytes, so the last `img.h*4` bytes are whatever `malloc` returned. The
   harness therefore compares exactly the `w*h*4` bytes the C defines, and
   forks the two children back-to-back from an identical heap so that even the
   C's reads of uninitialised memory are reproducible.
