# CONFIGS.md — Phase A configuration-surface table

Derived mechanically from the branches `c_src/src/lib.c` actually takes. The
public ABI has **two entry points** plus **seven writable exported globals**
that feed the decoder's tables, so "configuration" here means:

* the DEFLATE stream structure `cp_inflate` switches on, and
* the `bpp`/`w`/`h` shape `convert_pix` switches on, and
* the contents of the exported globals a caller may mutate.

## Axes the C code branches on

| axis | values | where the C branches |
|------|--------|----------------------|
| `btype` | 0 stored, 1 fixed, 2 dynamic (3 = error, see ERRORS.md #6) | `cp_inflate` `switch (btype)` |
| `bfinal` | 1 (single block) / 0 then 1 (multi-block `do/while(!bfinal)`) | `cp_inflate` loop |
| `in` pointer alignment | `first_bytes = ((in+3)&~3) - in` ∈ {0,1,2,3} | `cp_inflate` prologue; selects `s->words`, `word_count` |
| trailing partial word | `last_bytes = (in_bytes - first_bytes) & 3` ∈ {0,1,2,3} → `final_word_available` | `cp_inflate` prologue, `cp_peak_bits` `else if` branch |
| `in_bytes` magnitude | 0, 1..3 (no whole word), 4, >4 | `word_count = (in_bytes-first_bytes)/4` |
| `out_bytes` slack | exact fit / slack / too small (error rows) | `s->out_end` checks in `cp_block` |
| decode path in `cp_block` | literal (`sym<256`), end-of-block (`sym==256`), match (`sym>256`) | `cp_block` if/else-if/else |
| match copy path | `backwards_distance == 1` → `memset`; else byte loop | `cp_block` `switch (backwards_distance)` |
| length symbol | 257..285 → `cp_len_base[0..28]`, extra bits 0..5 | `cp_len_extra_bits` / `cp_len_base` |
| distance symbol | 0..29 → `cp_dist_base[0..29]`, extra bits 0..13 | `cp_dist_extra_bits` / `cp_dist_base` |
| `cp_build` lookup fast path | `len <= 9` populates `s->lookup`; `len > 9` does not; `s == 0` skips `lookup` entirely | `cp_build` `if (s && len <= 9)` |
| dynamic header sizes | `nlit` 257..288, `ndst` 1..32, `nlen` 4..19 | `cp_dynamic` reads 5/5/4 bits |
| code-length repeats | symbol 16 (copy prev, 3-6), 17 (zeros, 3-10), 18 (zeros, 11-138), literal 0..15 | `cp_dynamic` `switch (sym)` |
| stored `LEN` | 0, 1, 2..3, ≥4, up to 65535 | `cp_stored` `memcpy` length |
| `bpp` | 1 (grey), 2 (grey+A), 3 (RGB), 4 (RGBA), other (no-op) | `convert_pix` `switch (bpp)` |
| `w`, `h` | 0, 1, 2, many | `convert_pix` loop bounds |
| exported globals | pristine / mutated by caller | all table lookups go through the `D` globals |

## Rows — one per combination the C treats differently

Every row is exercised with **many randomized inputs** (fixed seed
`0x5EED_C0DE_1234_5678`, xorshift64\*), calling both `.so`s through
`libloading` and comparing the full output buffer, the return code, and
`cp_error_reason` byte-for-byte.

### `cp_inflate` — stored blocks (btype 0)

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|---|---|---|---|
| 1 | `cp_inflate` | btype=0, `LEN=0`, single final block, `out_bytes=0` | `row01_stored_len0` | [x] |
| 2 | `cp_inflate` | btype=0, `LEN` 1..3 (payload smaller than one word), exact `out_bytes` | `row02_stored_len_1_to_3` | [x] |
| 3 | `cp_inflate` | btype=0, `LEN` 4..64, exact `out_bytes` | `row03_stored_len_4_to_64` | [x] |
| 4 | `cp_inflate` | btype=0, `LEN` 256..4096, slack `out_bytes` | `row04_stored_len_large` | [x] |
| 5 | `cp_inflate` | btype=0, all four `in` alignments (`first_bytes` 0,1,2,3) × all four `last_bytes` (0,1,2,3) | `row05_stored_alignment_x_lastbytes` | [x] |

### `cp_inflate` — fixed Huffman (btype 1)

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|---|---|---|---|
| 6 | `cp_inflate` | btype=1, literals only, all bytes 0..143 (8-bit codes) | `row06_fixed_literals_low_range` | [x] |
| 7 | `cp_inflate` | btype=1, literals only, all bytes 144..255 (9-bit codes) | `row07_fixed_literals_high_range` | [x] |
| 8 | `cp_inflate` | btype=1, literals only, random full 0..255 range | `row08_fixed_literals_full_range` | [x] |
| 9 | `cp_inflate` | btype=1, matches with `backwards_distance == 1` (the `memset` path) | `row09_fixed_match_distance_one` | [x] |
| 10 | `cp_inflate` | btype=1, matches with `backwards_distance > 1` (the byte-copy path), distance ≤ 4 (overlapping) | `row10_fixed_match_small_distance` | [x] |
| 11 | `cp_inflate` | btype=1, matches with large distance (257..32768, dist symbols 16..29, 7..13 extra bits) | `row11_fixed_match_large_distance` | [x] |
| 12 | `cp_inflate` | btype=1, every length symbol 257..285 (`len_base` 3..258, extra bits 0..5) exercised | `row12_fixed_every_length_symbol` | [x] |
| 13 | `cp_inflate` | btype=1, every distance symbol 0..29 exercised | `row13_fixed_every_distance_symbol` | [x] |
| 14 | `cp_inflate` | btype=1, mixed literals + matches, randomized, exact `out_bytes` | `row14_fixed_mixed_random` | [x] |
| 15 | `cp_inflate` | btype=1, all four `in` alignments × all four `last_bytes` | `row15_fixed_alignment_x_lastbytes` | [x] |

### `cp_inflate` — dynamic Huffman (btype 2)

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|---|---|---|---|
| 16 | `cp_inflate` | btype=2, minimum header (`nlit=257`, `ndst=1`, `nlen=4`), literals only | `row16_dynamic_min_header` | [x] |
| 17 | `cp_inflate` | btype=2, maximum header (`nlit=288`, `ndst=32`, `nlen=19`) | `row17_dynamic_max_header` | [x] |
| 18 | `cp_inflate` | btype=2, code lengths encoded **without** any repeat symbol (16/17/18 absent) | `row18_dynamic_no_repeat_symbols` | [x] |
| 19 | `cp_inflate` | btype=2, code lengths using repeat symbol 16 (copy-previous 3..6) | `row19_dynamic_repeat16` | [x] |
| 20 | `cp_inflate` | btype=2, code lengths using repeat symbol 17 (short zero run 3..10) | `row20_dynamic_repeat17` | [x] |
| 21 | `cp_inflate` | btype=2, code lengths using repeat symbol 18 (long zero run 11..138) | `row21_dynamic_repeat18` | [x] |
| 22 | `cp_inflate` | btype=2, symbol code lengths > 9 present (defeats the `len <= 9` `lookup` fast path) | `row22_dynamic_lengths_above_nine` | [x] |
| 23 | `cp_inflate` | btype=2, literals + matches, randomized alphabet, randomized data | `row23_dynamic_mixed_random` | [x] |
| 24 | `cp_inflate` | btype=2, all four `in` alignments × all four `last_bytes` | `row24_dynamic_alignment_x_lastbytes` | [x] |

### `cp_inflate` — multi-block / mixed

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|---|---|---|---|
| 25 | `cp_inflate` | 2..6 blocks, all btype=1 | `row25_multiblock_all_fixed` | [x] |
| 26 | `cp_inflate` | 2..6 blocks, all btype=2 | `row26_multiblock_all_dynamic` | [x] |
| 27 | `cp_inflate` | 2..6 blocks, randomly mixed btype ∈ {1,2} (stored excluded: see note) | `row27_multiblock_mixed_types` | [x] |
| 28 | `cp_inflate` | matches that reach back across a block boundary (`begin`-relative distance) | `row28_match_across_block_boundary` | [x] |
| 29 | `cp_inflate` | real zlib-produced streams (`flate2`, levels 1/6/9, deflate-raw) over randomized/compressible/incompressible data — exercises whatever block mix the encoder picks | `row29_real_zlib_streams` | [x] |
| 30 | `cp_inflate` | `out_bytes` exactly equal to, and larger than, the decompressed size | `row30_out_bytes_exact_and_slack` | [x] |

Note on row 27: a stored block can only ever be the **last** block a C caller can
feed, for two independent reasons. First, `cp_stored` requires
`s->bits_left/8 <= LEN` (ERRORS.md #2), so any bytes after the stored payload make
the C reject the stream. Second, `cp_stored` byte-aligns with
`cp_read_bits(s, s->count & 7)` rather than `bits_left & 7`; that is only
equivalent to a real byte alignment while the reader's
`bits_left ≡ count (mod 8)` invariant holds, which a *preceding* Huffman block can
break — and then `cp_ptr` aborts (ERRORS.md #11). Stored blocks are therefore
tested single/final in rows 1-5, the stored-after-Huffman case is covered as an
abort row by `err11_cp_ptr_alignment_assert`, and row 27 mixes btype ∈ {1,2}.
Real zlib streams that end in a stored block are covered by row 29, which
compares both the padded and unpadded forms.

## Verification status

All 46 rows pass across their randomized inputs. Reproduce with:

```
cd translation
cargo build --release
cargo test --release --test phase_b_valid -- --test-threads=1
```

`--test-threads=1` is required: rows 31-35 mutate the libraries' exported global
tables, which are shared process-wide once the `.so` is `dlopen`ed.

### `cp_inflate` — mutated exported globals

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|---|---|---|---|
| 31 | `cp_inflate` + `cp_len_base` | caller rewrites `cp_len_base[k]`; both must decode with the mutated base | `row31_mutated_len_base` | [x] |
| 32 | `cp_inflate` + `cp_dist_base` | caller rewrites `cp_dist_base[k]` | `row32_mutated_dist_base` | [x] |
| 33 | `cp_inflate` + `cp_len_extra_bits` / `cp_dist_extra_bits` | caller rewrites the extra-bit counts (still ≤ 13) | `row33_mutated_extra_bit_counts` | [x] |
| 34 | `cp_inflate` + `cp_permutation_order` | caller permutes `cp_permutation_order` (btype=2 header order changes) | `row34_mutated_permutation_order` | [x] |
| 35 | `cp_inflate` + `cp_fixed_table` | caller rewrites `cp_fixed_table` entries (values 1..15) → different fixed tree | `row35_mutated_fixed_table` | [x] |
| 36 | `cp_error_reason` | global is readable/writable from the caller and reads back what the library stored | `row36_error_reason_global_roundtrip` | [x] |

### `convert_pix`

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|---|---|---|---|
| 37 | `convert_pix` | `bpp=1`, `w`×`h` ∈ {1×1, 1×3, 3×1, 7×5, 64×17}, randomized bytes | `row37_convert_pix_bpp1` | [x] |
| 38 | `convert_pix` | `bpp=2`, same shape matrix, randomized bytes | `row38_convert_pix_bpp2` | [x] |
| 39 | `convert_pix` | `bpp=3`, same shape matrix, randomized bytes | `row39_convert_pix_bpp3` | [x] |
| 40 | `convert_pix` | `bpp=4`, same shape matrix, randomized bytes | `row40_convert_pix_bpp4` | [x] |
| 41 | `convert_pix` | `bpp` ∈ {1,2,3,4} with all-`0x00` and all-`0xFF` payloads (alpha/luma boundaries) | `row41_convert_pix_boundary_values` | [x] |
| 42 | `convert_pix` | `bpp` ∈ {1,2,3,4}, `w=0` (row-skip byte still consumed per row) | `row42_convert_pix_w_zero` | [x] |
| 43 | `convert_pix` | `bpp` ∈ {1,2,3,4}, `h=0` (no bytes touched) | `row43_convert_pix_h_zero` | [x] |
| 44 | `convert_pix` | `bpp` ∈ {1,2,3,4}, `h=1`, `w=1` (single pixel, minimum shape) | `row44_convert_pix_single_pixel` | [x] |

### End-to-end pipeline (the composed path a real consumer drives)

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|---|---|---|---|
| 45 | `cp_inflate` → `convert_pix` | deflate a PNG-style filtered scanline buffer (`h` rows of `1 + w*bpp` bytes) with `flate2`, inflate it through both `.so`s, then run `convert_pix` on the inflated buffer, for `bpp` ∈ {1,2,3,4} × randomized `w`,`h` | `row45_pipeline_via_zlib` | [x] |
| 46 | `cp_inflate` → `convert_pix` | same, but the inflate is driven from a hand-built btype=2 stream so the low-level dynamic path feeds `convert_pix` | `row46_pipeline_via_handbuilt_dynamic` | [x] |

## Binary / driver

`c_src/CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)` — there
is **no executable target**, and the Rust crate is `crate-type = ["cdylib"]`
only. The "compare C and Rust binary stdout" gate is **N/A** for this project.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default one. This is verified mechanically rather than
assumed: `scripts/verify_all.sh` enumerates features via
`cargo metadata --no-deps`, builds the full power set of combinations (plus the
`--no-default-features` baseline), and for each one re-runs the `nm -D` symbol
diff and every test suite. Its output for this crate is
`no [features] declared -> the only configuration is the default one` /
`combinations to verify: 1`.

## Harness credibility

`scripts/mutation_check.py` injects nine small behavioural changes into
`src/lib.rs` and confirms the suite catches them (8 detected; the 9th, in
`cp_paeth`, is unreachable across the ABI because its only caller `cp_unfilter`
is `static` with no call sites). This is what rules out the tables above being
satisfied by vacuous tests.
