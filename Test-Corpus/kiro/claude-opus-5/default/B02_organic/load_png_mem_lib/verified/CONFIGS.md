# CONFIGS.md — configuration-surface table (valid inputs)

Derived mechanically from the `if`/`switch` branches in `c_src/src/lib.c` plus
the public surface of `c_src/include/lib.h`. There are no `#ifdef`s and no
Cargo features, so the axes are all runtime.

## Axes the C actually branches on

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| entry point | `cp_inflate` (low level), `load_png_mem` (high level) | `lib.h` + `nm -D` |
| DEFLATE block type (`btype`) | `0` stored, `1` fixed Huffman, `2` dynamic Huffman | `cp_inflate` switch L332 |
| block count | single `BFINAL=1` block, multiple blocks of mixed type | `do { ... } while(!bfinal)` |
| input pointer alignment `in & 3` | `0,1,2,3` → `first_bytes` 0..3, changes the pre-load path | `cp_inflate` L305 |
| input tail `(in_bytes-first_bytes) & 3` | `0,1,2,3` → `final_word_available` 0/1 | `cp_inflate` L311 |
| `cp_build` with/without lookup table | `s != NULL` (lit table, fills `s->lookup`) vs `s == NULL` (dst/len tables) | `cp_build` L127/L143 |
| dynamic-header code-length symbols | literal 0..15, `16` (copy prev 3–6), `17` (zeros 3–10), `18` (zeros 11–138) | `cp_dynamic` switch L215 |
| `HCLEN`/`HLIT`/`HDIST` | `nlen` 4..19, `nlit` 257..288, `ndst` 1..32 | `cp_dynamic` L209-211 |
| back-reference distance | `== 1` (memset fast path) vs `> 1` (byte copy loop), overlapping vs non-overlapping | `cp_block` switch L286 |
| match length | 3..258 incl. the `len_base` boundary entries (symbol 284 → 227+extra, symbol 285 → 258/0 extra bits) | `cp_len_base` |
| PNG colour type | `0` grey (bpp 1), `2` RGB (bpp 3), `3` indexed (bpp 1), `4` grey+alpha (bpp 2), `6` RGBA (bpp 4) | `load_png_mem` switch L553 |
| PNG per-scanline filter | `0` none, `1` sub, `2` up, `3` average, `4` paeth — **and** row 0 is special-cased (`2` is a no-op, `1`/`3`/`4` start at `x=bpp`) | `cp_unfilter` two switches |
| `PLTE` chunk | absent, present (used only when colour type 3) | `cp_find` L646 |
| `tRNS` chunk | absent; present with `trns_len > max index` (all alphas used); present with `trns_len <= some index` (falls back to 255) | `cp_get_alpha_for_indexed_image` |
| IDAT layout | one chunk; many chunks that must be concatenated; chunk interleaved with unknown chunks | the two `cp_find`/`cp_chunk` IDAT loops |
| chunk order | `PLTE` before `tRNS` before `IDAT` (the only order the C's forward-only scan can find) | `first`/`png.p` rewind logic L644-656 |
| image shape | `1x1`, `1xN` (single column), `Nx1` (single row), `NxM`, widths crossing the `bpp` boundary | `w`,`h` |
| zlib header | `CMF` low nibble `8`; `CINFO` `0x00..0x70`; `FLG` without `0x20` | L680-700 |

## Rows (pruned cross-product — combinations the C treats differently)

### `cp_inflate` direct (low-level entry point)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `cp_inflate` | btype=0 stored, single final block, `in&3 == 0`, tail 0, LEN matching `bits_left/8` | [x] |
| 2 | `cp_inflate` | btype=0 stored, `in&3 == 1,2,3` (all three `first_bytes` pre-load paths) | [x] |
| 3 | `cp_inflate` | btype=0 stored, `LEN == 0` (empty stored block) | [x] |
| 4 | `cp_inflate` | btype=1 fixed Huffman, literals only, out buffer exactly sized | [x] |
| 5 | `cp_inflate` | btype=1 fixed, literals + length/distance with `distance == 1` (memset path) | [x] |
| 6 | `cp_inflate` | btype=1 fixed, length/distance with `distance > 1`, overlapping copy (`distance < length`) | [x] |
| 7 | `cp_inflate` | btype=1 fixed, length/distance non-overlapping (`distance >= length`) | [x] |
| 8 | `cp_inflate` | btype=1 fixed, literals ≥ 144 (the 9-bit half of `cp_fixed_table`) | [x] |
| 9 | `cp_inflate` | btype=1 fixed, max match length 258 (symbol 285, 0 extra bits) and 227..257 (symbol 284, 5 extra bits) | [x] |
| 10 | `cp_inflate` | btype=1 fixed, max distance symbols 29/30 (large `dist_extra_bits`) | [x] |
| 11 | `cp_inflate` | btype=2 dynamic, code-length alphabet using only literal lengths (no 16/17/18) | [x] |
| 12 | `cp_inflate` | btype=2 dynamic, code-length symbol `16` (repeat previous) exercised | [x] |
| 13 | `cp_inflate` | btype=2 dynamic, code-length symbol `17` (short zero run) exercised | [x] |
| 14 | `cp_inflate` | btype=2 dynamic, code-length symbol `18` (long zero run, 11..138) exercised | [x] |
| 15 | `cp_inflate` | btype=2 dynamic, `ndst == 1` (single distance code) | [x] |
| 16 | `cp_inflate` | btype=2 dynamic, `nlit == 288`, `ndst == 32`, `nlen == 19` (maximum header) | [x] |
| 17 | `cp_inflate` | multi-block: stored → fixed → dynamic → final | [x] |
| 18 | `cp_inflate` | multi-block: 2+ fixed blocks with a back-reference crossing the block boundary | [x] |
| 19 | `cp_inflate` | all 4 combinations of `(in&3, in_bytes&3)` tails so `final_word_available` is 0 and 1 | [x] |
| 20 | `cp_inflate` | out buffer larger than needed (partial fill) | [x] |
| 21 | `cp_inflate` | randomized raw payloads (0..4 KiB) round-tripped through a reference deflate encoder, all three block types, fixed seed | [x] |

### `load_png_mem` (high-level entry point)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 22 | `load_png_mem` | colour type 0 (grey, bpp 1), filter 0 on every row, 1x1 | [x] |
| 23 | `load_png_mem` | colour type 0, filters 0..4 mixed per row, `NxM` | [x] |
| 24 | `load_png_mem` | colour type 2 (RGB, bpp 3), filters 0..4 mixed, `NxM` | [x] |
| 25 | `load_png_mem` | colour type 4 (grey+alpha, bpp 2), filters 0..4 mixed, `NxM` | [x] |
| 26 | `load_png_mem` | colour type 6 (RGBA, bpp 4), filters 0..4 mixed, `NxM` | [x] |
| 27 | `load_png_mem` | colour type 3 (indexed, bpp 1) + PLTE, no tRNS | [x] |
| 28 | `load_png_mem` | colour type 3 + PLTE + tRNS with `trns_len` covering every index | [x] |
| 29 | `load_png_mem` | colour type 3 + PLTE + tRNS with `trns_len` **shorter** than the max index used (255-fallback branch) | [x] |
| 30 | `load_png_mem` | colour type 3 + PLTE shorter than 256 entries (C reads past PLTE — must agree) | [x] |
| 31 | `load_png_mem` | row-0 filter `1` (sub, starts at `x=bpp`), all bpp | [x] |
| 32 | `load_png_mem` | row-0 filter `2` (up — no-op on row 0), all bpp | [x] |
| 33 | `load_png_mem` | row-0 filter `3` (average, `raw[x-bpp]/2` only), all bpp | [x] |
| 34 | `load_png_mem` | row-0 filter `4` (paeth with `b=c=0`), all bpp | [x] |
| 35 | `load_png_mem` | rows ≥1 filter `3` (average with both neighbours, int rounding) | [x] |
| 36 | `load_png_mem` | rows ≥1 filter `4` (full paeth predictor, all three predictor branches) | [x] |
| 37 | `load_png_mem` | shape `1x1` | [x] |
| 38 | `load_png_mem` | shape `1xN` (single column, many rows) | [x] |
| 39 | `load_png_mem` | shape `Nx1` (single row) | [x] |
| 40 | `load_png_mem` | shape where `w*bpp` is not a multiple of 4 (unaligned scanlines) | [x] |
| 41 | `load_png_mem` | IDAT split across 2..5 chunks | [x] |
| 42 | `load_png_mem` | IDAT preceded by an unknown ancillary chunk (`gAMA`) so `cp_find` must skip it | [x] |
| 43 | `load_png_mem` | zlib `CINFO` at each of `0x00,0x10,...,0x70` (all accepted window sizes) | [x] |
| 44 | `load_png_mem` | zlib `FLG` varying in the bits the C ignores (FCHECK/FLEVEL) | [x] |
| 45 | `load_png_mem` | IDAT payload built with btype 0 / 1 / 2 (stored / fixed / dynamic) | [x] |
| 46 | `load_png_mem` | trailing `IEND` chunk present vs absent | [x] |
| 47 | `load_png_mem` | randomized images: random `w,h` in 1..24, random colour type from {0,2,3,4,6}, random per-row filters 0..4, random pixel data, fixed seed, 400+ cases | [x] |
| 48 | `load_png_mem` | randomized indexed images with random PLTE/tRNS lengths, fixed seed | [x] |
| 49 | `load_png_mem` | large-ish image (256x256 RGBA) to exercise multi-word inflate and long matches | [x] |

### Not applicable

* No `[[bin]]` / `src/main.rs` in the crate and no `add_executable` in
  `c_src/CMakeLists.txt` → **no driver binary**, so no stdout comparison.
* No `[features]` in `Cargo.toml` → one configuration only.

---

# Row → test mapping (Phase B result)

Every row is exercised by a differential test that calls **both** `.so`s through
their exported symbols and compares byte-for-byte. Randomised rows use a fixed
seed. Each of the 49 rows above is checked off because its test passes; the
mapping is:

| rows | test | scale |
|------|------|-------|
| 1, 3 | `phase_b_inflate::row01_stored_aligned`, `row03_stored_empty` | 13 lengths |
| 2 | `row02_stored_all_input_alignments` | 10 lengths × 4 alignments |
| 4 | `row04_fixed_literals_only` | 7 sizes + all 256 literal values |
| 5 | `row05_fixed_match_distance_one` | 8 lengths |
| 6 | `row06_fixed_overlapping_copy` | 6 (len,dist) pairs |
| 7 | `row07_fixed_non_overlapping_copy` | 5 (len,dist) pairs |
| 8 | `row08_fixed_nine_bit_literals` | all of 144..255 + interleaved |
| 9 | `row09_fixed_length_symbol_boundaries` | every length symbol + both ends of every extra-bit range |
| 10 | `row10_fixed_distance_symbol_boundaries` | all 30 distance symbols × both ends |
| 11 | `row11_dynamic_literal_code_lengths_only` | 4 sizes |
| 12, 13, 14 | `row12_dynamic_cl_symbol_16`, `row13_..._17`, `row14_..._18` | each test asserts the vector really uses that CL symbol before running |
| 15 | `row15_dynamic_ndst_one` | both CL encodings |
| 16 | `row16_dynamic_maximum_header` | `nlit=288, ndst=32`, HCLEN swept 4..19 |
| 17 | `row17_multiblock_fixed_dynamic_stored` | fixed → dynamic → stored(final) |
| 18 | `row18_multiblock_backref_across_boundary` | 3 blocks, back-references crossing both boundaries |
| 19 | `row19_input_alignment_and_tail_matrix` | 4 alignments × 4 length residues |
| 20 | `row20_output_buffer_larger_than_needed` | 3 block types × 5 slack sizes |
| 21 | `row21_randomized_payloads_all_block_types` (300 payloads × 5 encodings), `row21b_randomized_token_streams` (300 × 3), `row21c_randomized_multiblock` (120) | ~2400 streams |
| 22–26 | `phase_b_png::row22_...` … `row26_rgba_mixed_filters` | every colour type × 4–5 shapes × all 6 DEFLATE encodings |
| 27–30 | `row27_indexed_no_trns`, `row28_indexed_trns_covers_all_indices`, `row29_indexed_trns_shorter_than_max_index`, `row30_indexed_short_palette` | tRNS lengths 0..41, palettes of 1..255 entries |
| 31–34 | `rows31_34_row0_filters` | 5 filters × 5 colour types × 5 widths |
| 35, 36 | `rows35_36_later_row_filters`, `row36_paeth_all_predictor_branches` | 5 filters × 4 colour types × 4 shapes; 44 paeth patterns |
| 37–40 | `rows37_40_shapes` | 12 shapes × 4 colour types, incl. `w*bpp` not a multiple of 4 |
| 41 | `row41_idat_split_across_chunks` | 1..6 IDAT chunks |
| 42 | `row42_ancillary_chunk_before_idat` | gAMA + cHRM + sRGB skipped by `cp_find` |
| 43 | `row43_zlib_cinfo_sweep` | CINFO 0..7 |
| 44 | `row44_zlib_flg_ignored_bits` | 8 FLG values |
| 45 | `row45_all_deflate_block_types` | 4 colour types × 6 encodings |
| 46 | `row46_iend_present_or_absent` | both |
| 47 | `row47_randomized_images` | 420 random images (random `w,h` ≤ 24, random colour type, random per-row filters, random IDAT split, random encoding) |
| 48 | `row48_randomized_indexed_with_palette_and_trns` | 200 random indexed images |
| 49 | `row49_large_image` | 256×256 RGBA × 3 encodings, plus 4096×1 and 1×4096 |

## Non-vacuity

Each Phase B test first runs the vector through the **C** `.so` alone and
asserts the C accepts it *and* decodes it to a plaintext / pixel array computed
independently of both implementations (`common::deflate::apply` for DEFLATE
token streams, `expected_pixels` for PNGs). Only then is the Rust `.so` compared
against the C. So a bug in the generator cannot make the two "agree on garbage".

Two places relax this deliberately, and say so in the test:

* **Stored DEFLATE blocks.** `cp_stored` derives the source pointer from
  `cp_ptr`, whose bit accounting is off by a byte for some input lengths (the
  final-word path in `cp_peak_bits` adds `bits_left` rather than
  `last_bytes * 8`). The C therefore copies from the wrong offset. Those rows
  assert "C accepts, and Rust matches C exactly" (`ok_loose`) rather than
  "C produces the plaintext" — the C is ground truth.
* **Short PLTE** (row 30). `cp_depalette` indexes `plte[c*3]` with no bound on
  `c`, so a short palette makes the C read the bytes following the chunk. Both
  libraries are handed an identical, deterministically padded buffer, so the
  requirement is agreement.

## Additional coverage beyond the table

`tests/isolated.rs` adds 3694 subprocess cases that also fall under Phase B for
the ones that succeed, including 26 cases where a run-length code overruns the
C's `uint8_t lens[288+32]` into its own stack frame by up to 35 bytes, the C
still decodes the block, and the Rust translation's emulated frame produces
byte-identical output. That is the positive test for the most delicate part of
the translation.
