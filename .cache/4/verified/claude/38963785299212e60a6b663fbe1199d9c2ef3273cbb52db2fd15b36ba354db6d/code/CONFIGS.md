# CONFIGS.md — configuration-surface table (Phase A / Phase B)

## Build-time configuration

* `c_src/CMakeLists.txt` has **no** `option()`, no `target_compile_definitions`,
  and there is no `#if`/`#ifdef`/`#ifndef` anywhere in `src/lib.c` or
  `include/lib.h`:

  ```sh
  $ grep -rn '#if\|#ifdef\|#ifndef\|option(\|_DEFINITIONS\|NDEBUG' \
        c_src/include c_src/src c_src/CMakeLists.txt ; echo $?
  1        # nothing found
  ```

  The only build-time knob that matters is `NDEBUG`, and it is **not** defined
  (no `CMAKE_BUILD_TYPE` is set), so `assert()` is live — see `ERRORS.md` §C.
* Consequently `Cargo.toml` declares `[features] default = []`. The complete set
  of valid feature combinations is therefore a single, empty one. `verify.sh`
  enumerates it from `Cargo.toml` (it computes the power set of the declared
  optional features) rather than hard-coding it:

  | # | feature combination | `cargo check --all-targets` | `cargo test` (dev) | `cargo test --release` |
  |---|---------------------|---------------------------|--------------------|------------------------|
  | F1 | `--no-default-features` (empty) | ok | ok, 92 tests | ok, 92 tests |
  | F2 | `--all-features` (identical to F1) | ok | ok, 92 tests | ok, 92 tests |

  Both cargo profiles are covered: `dev` (overflow checks **on**,
  `panic=unwind`) and `release` (`panic=abort`, optimised). Symbol parity is
  checked for both `.so`s.

## Run-time configuration axes actually branched on by the C code

| axis | source | values |
|------|--------|--------|
| colour type → `bpp` | `lib.c:555-580` | `0→1`, `2→3`, `3→1`, `4→2`, `6→4` |
| indexed vs direct | `lib.c:738` | `cp_depalette` (colour type 3) vs `cp_convert` |
| `cp_convert` pixel builder | `lib.c:468-481` | `switch (bpp)` = 1, 2, 3, 4 |
| alpha of an indexed pixel | `lib.c:485-493` | `trns == NULL` / `index >= trns_len` / else |
| row-0 filter | `lib.c:407-426` | 0, 1 (starts at `x=bpp`), 2 (**no-op**), 3 (`raw[x-bpp]/2`), 4 (`paeth(a,0,0)`) |
| row-`y>0` filter | `lib.c:431-460` | 0, 1, 2, 3, 4 (full Paeth) |
| back-reference copy | `lib.c:284-291` | `distance == 1` → `memset`, else a byte loop |
| DEFLATE block type | `lib.c:324-354` | 0 stored, 1 fixed, 2 dynamic |
| block chaining | `lib.c:321-356` | single (`BFINAL=1`) vs several blocks |
| bit-buffer refill | `lib.c:84-95` | from `words[]` vs from `final_word` |
| input alignment | `lib.c:305-315` | `first_bytes` 0..3 |
| input tail | `lib.c:308-314` | `last_bytes` 0..3 (`final_word_available` 0/1) |
| code-length RLE | `lib.c:218-234` | literal length vs symbol 16 / 17 / 18 |
| chunk scan | `lib.c:377-401` | `cp_chunk` (must be the *next* chunk) vs `cp_find` (search) |
| chunk order | `lib.c:647-672` | the `first`/`png.p` rewind dance for PLTE / tRNS / IDAT |
| zlib header | `lib.c:682-704` | `CM`, `CINFO` 0..7, `FLEVEL`, `FCHECK`, `FDICT=0` |

Every row below is driven by **many randomised inputs** from a fixed-seed
`splitmix64` (`tests/common/mod.rs::Rng`), not a single hand-picked value, and
successful decodes are additionally cross-checked against an **independent
RFC 2083 reference decoder** (`reference_rgba`) so that "both wrong the same
way" cannot pass as success.

Status: **76 / 76 rows pass** (`tests/inflate.rs` 18 fns, `tests/png.rs` 24 fns,
`tests/chunks.rs` 6 fns, `tests/symbols.rs` 3 fns, plus `tests/fuzz.rs` 4 property
tests).

### `cp_inflate` — the lowest-level public entry point (`tests/inflate.rs`)

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `cp_inflate` | bt=1 fixed, literals only, in-ptr **align 0** (`first_bytes=0`), 40 random payloads | `row_1_4_input_alignment` | [x] |
| 2 | `cp_inflate` | bt=1, in-ptr **align 3** (`first_bytes=1`) | `row_1_4_input_alignment` | [x] |
| 3 | `cp_inflate` | bt=1, in-ptr **align 2** (`first_bytes=2`) | `row_1_4_input_alignment` | [x] |
| 4 | `cp_inflate` | bt=1, in-ptr **align 1** (`first_bytes=3`) | `row_1_4_input_alignment` | [x] |
| 5 | `cp_inflate` | `in_bytes-first_bytes ≡ 0 (mod 4)` → `last_bytes=0`, `final_word_available=0` | `row_5_8_input_tail` | [x] |
| 6 | `cp_inflate` | `≡ 1 (mod 4)` → `last_bytes=1`, `final_word_available=1` | `row_5_8_input_tail` | [x] |
| 7 | `cp_inflate` | `≡ 2 (mod 4)` → `last_bytes=2` | `row_5_8_input_tail` | [x] |
| 8 | `cp_inflate` | `≡ 3 (mod 4)` → `last_bytes=3` | `row_5_8_input_tail` | [x] |
| 9 | `cp_inflate` | full cross product align(4) x tail(4) = 16 combinations, coverage asserted | `row_9_alignment_tail_cross_product` | [x] |
| 10 | `cp_inflate` | bt=0 stored, single block, `LEN ∈ {0,1,2,3,4,5,7,8,15,16,17,63,64,255,256,1023,4096}` + 25 random, x 4 alignments. **Note the C quirk** (see below) | `row_10_stored_block` | [x] |
| 11 | `cp_inflate` | bt=0 stored, `LEN == 0` (`memcpy(dst,src,0)`), `out_bytes ∈ {0,1,16}` | `row_11_stored_empty` | [x] |
| 12 | `cp_inflate` | bt=2 dynamic, flate2 level 1 (x 4 alignments, random + repetitive payloads) | `row_12_14_dynamic_flate2` | [x] |
| 13 | `cp_inflate` | bt=2 dynamic, flate2 level 6 | `row_12_14_dynamic_flate2` | [x] |
| 14 | `cp_inflate` | bt=2 dynamic, flate2 levels 0, 2-5, 7-9 | `row_12_14_dynamic_flate2` | [x] |
| 15 | `cp_inflate` | bt=2 dynamic from our **own** encoder, with and without the code-length RLE symbols 16/17/18, so `cp_dynamic`'s HLIT/HDIST/HCLEN + permutation-order + run-length grammar is driven directly (60 iterations x 2 modes x 4 alignments) | `row_15_dynamic_handrolled` | [x] |
| 16 | `cp_inflate` | 2..5 fixed blocks, `BFINAL` only on the last | `row_16_multi_fixed_blocks` | [x] |
| 17 | `cp_inflate` | mixed fixed + dynamic block chains (incl. fixed→dynamic→fixed) | `row_17_mixed_block_types` | [x] |
| 18 | `cp_inflate` | back-reference **distance == 1** → `memset` branch, **every length 3..=258** plus a spot check of both encoders x 4 alignments | `row_18_distance_one_memset` | [x] |
| 19 | `cp_inflate` | back-reference **distance >= length**, 200 random (len, dist) pairs | `row_19_nonoverlapping_matches` | [x] |
| 20 | `cp_inflate` | **overlapping** back-references: distance 2..8 x **every** length 3..=258 | `row_20_overlapping_matches` | [x] |
| 21 | `cp_inflate` | **every length code 257..285** and every extra-bit value of each (all `cp_len_base` / `cp_len_extra_bits` entries), both encoders | `row_21_all_length_codes` | [x] |
| 22 | `cp_inflate` | **every distance code 0..29**, sampled extra-bit values plus each code's maximum, distances up to 32768 | `row_22_all_distance_codes` | [x] |
| 23 | `cp_inflate` | literal alphabet: **each of the 256 literals alone**, the whole alphabet forwards and backwards, and the 8-/9-bit code boundary (142,143,144,145) | `row_23_literal_alphabet` | [x] |
| 24 | `cp_inflate` | `out_bytes` larger than the stream produces; the slack tail is asserted to stay `0xCD` | `row_24_out_bytes_slack` | [x] |
| 25 | `cp_inflate` | output > 64 KiB (65536, 70000, 131072) → several flate2 blocks, distances up to 32768 | `row_25_large_output` | [x] |
| 26 | `cp_inflate` | 1-byte output x 4 alignments x fixed / stored / dynamic | `row_26_single_byte` | [x] |

> **C quirk pinned by row 10.** `cp_stored` recovers the source pointer with
> `cp_ptr() = (char*)(words + word_index) - count/8`, which is only exact while
> the bit buffer has been refilled from `words[]` alone. As soon as the 40 header
> bits have to be completed from `s->final_word` (which adds `bits_left`, not 32,
> to `count`) the pointer is off by `bits_left/8` bytes and the C library copies
> the **wrong bytes**. Measured against the C `.so` for `LEN = 0..6` x
> `align = 0..3`, the content is correct exactly when `align=0 && LEN>=3`,
> `align=1 && LEN>=2`, `align=2 && LEN>=1`, or `align=3`. Outside that set the
> test only requires that Rust matches C byte for byte — which it does.

### `load_png_mem` — colour type / `bpp` surface (`tests/png.rs`)

| # | entry point(s) | configuration | test | [x] |
|---|----------------|---------------|------|-----|
| 27 | `load_png_mem` | ct=0 grey, bpp=1 → `cp_convert` case 1, 30 random sizes | `row_27_30_direct_colour_types` | [x] |
| 28 | `load_png_mem` | ct=2 RGB, bpp=3 → `cp_convert` case 3 | `row_27_30_direct_colour_types` | [x] |
| 29 | `load_png_mem` | ct=4 grey+alpha, bpp=2 → `cp_convert` case 2 | `row_27_30_direct_colour_types` | [x] |
| 30 | `load_png_mem` | ct=6 RGBA, bpp=4 → `cp_convert` case 4 | `row_27_30_direct_colour_types` | [x] |
| 31 | `load_png_mem` | ct=3 indexed, 256-entry PLTE, **no** tRNS → the `alpha = 255` branch | `row_31_indexed_no_trns` | [x] |
| 32 | `load_png_mem` | ct=3, PLTE + tRNS with `trns_len == 256` → the `trns[index]` branch | `row_32_36_indexed_trns_lengths` | [x] |
| 33 | `load_png_mem` | ct=3, `trns_len ∈ {1,2,17,128,255}` → mixed `trns[index]` / 255 | `row_32_36_indexed_trns_lengths` | [x] |
| 34 | `load_png_mem` | ct=3, **zero-length** tRNS (`trns_len == 0`) → always 255 | `row_32_36_indexed_trns_lengths` | [x] |
| 35 | `load_png_mem` | ct=3, PLTE **shorter** than the largest index (3, 6, 30, 300, 765 bytes) — `plte[c*3]` reads past the chunk; both libraries get the identical padded buffer | `row_35_short_palette` | [x] |
| 36 | `load_png_mem` | ct=3, tRNS **longer** than 256 (257, 300, 512) | `row_32_36_indexed_trns_lengths` | [x] |
| 37 | `load_png_mem` | ct ≠ 3 **with** PLTE and/or tRNS present → ignored by `cp_convert`, but they still move the internal `first` cursor | `row_37_palette_on_direct_colour_type` | [x] |

### `load_png_mem` — filter surface (`cp_unfilter`)

| # | entry point(s) | configuration | test | [x] |
|---|----------------|---------------|------|-----|
| 38 | `load_png_mem` | every row filter = 0 (None), all 5 colour types | `row_38_42_uniform_filters` | [x] |
| 39 | `load_png_mem` | every row filter = 1 (Sub); row 0 starts at `x=bpp`, rows>0 at `x=0` | `row_38_42_uniform_filters` | [x] |
| 40 | `load_png_mem` | every row filter = 2 (Up); **row 0 case 2 is a no-op in C** | `row_38_42_uniform_filters` | [x] |
| 41 | `load_png_mem` | every row filter = 3 (Average); row 0 `raw[x-bpp]/2`, rows>0 `(raw[x-bpp]+prev[x])/2` | `row_38_42_uniform_filters` | [x] |
| 42 | `load_png_mem` | every row filter = 4 (Paeth); row 0 `cp_paeth(a,0,0)`, rows>0 the full Paeth | `row_38_42_uniform_filters` | [x] |
| 43 | `load_png_mem` | a **random filter per row** (0..4), h up to 32, all colour types | `row_43_random_filters` | [x] |
| 44 | `load_png_mem` | `h == 1` → the `for (y=1; y<h; …)` loop never runs; all filters x all colour types x w ∈ {1,2,3,4,7,16,33} | `row_44_single_row` | [x] |
| 45 | `load_png_mem` | `w == 1` → `len == bpp`, so the `for (x=bpp; x<len; …)` loops never run either | `row_45_single_column` | [x] |

### `load_png_mem` — image-shape surface

| # | entry point(s) | configuration | test | [x] |
|---|----------------|---------------|------|-----|
| 46 | `load_png_mem` | 1x1, all 5 colour types x all 5 filters | `row_46_one_by_one` | [x] |
| 47 | `load_png_mem` | 1xN (N ∈ {1,2,3,5,13,64}), all colour types | `row_47_48_thin_images` | [x] |
| 48 | `load_png_mem` | Nx1, all colour types | `row_47_48_thin_images` | [x] |
| 49 | `load_png_mem` | `w ∈ {1,2}` with bpp ∈ {1,2} — `(w+1)·h·4 - (w+1)·h·bpp` exceeds `w·h·4`, so `out` sits *after* the converted pixels | `row_49_50_out_overlap_boundary` | [x] |
| 50 | `load_png_mem` | `w == 3` with bpp == 1 — the exact boundary `(w+1)·h·3 == w·h·4` | `row_49_50_out_overlap_boundary` | [x] |
| 51 | `load_png_mem` | 64x64, 127x33, 256x3, 3x256, 300x17, all colour types, flate2 payload | `row_51_larger_images` | [x] |
| 52 | `load_png_mem` | 300 randomised (w, h, colour type, per-row filter, compressor, IDAT split, tRNS length) combinations | `row_52_randomised_cross_product` + `fuzz_valid_pngs` (600 more) | [x] |

### `load_png_mem` — container / chunk surface

| # | entry point(s) | configuration | test | [x] |
|---|----------------|---------------|------|-----|
| 53 | `load_png_mem` | one IDAT chunk holding the whole zlib stream | `row_53_55_idat_splitting` | [x] |
| 54 | `load_png_mem` | the stream spread over 2..17 **contiguous** IDAT chunks | `row_53_55_idat_splitting` | [x] |
| 55 | `load_png_mem` | zero-length IDAT chunks interleaved with the real ones (x 1..17 splits) | `row_53_55_idat_splitting` | [x] |
| 56 | `load_png_mem` | **non-contiguous** IDATs: the complete stream in the leading run of 1..4 IDATs, then a `gAMA` chunk, then two junk IDATs that must be **ignored entirely** (`cp_chunk` demands the *next* chunk); also an unknown chunk *before* the first IDAT, which `cp_find` skips | `row_56_non_contiguous_idats` | [x] |
| 57 | `load_png_mem` | `gAMA`, `sRGB`, `tEXt`, `bKGD`, `pHYs` before the IDATs, all colour types | `row_57_58_unknown_chunks` | [x] |
| 58 | `load_png_mem` | the same unknown chunks *between* PLTE/tRNS and the IDATs, and in both positions at once | `row_57_58_unknown_chunks` | [x] |
| 59 | `load_png_mem` | tRNS **before** PLTE → `cp_find("PLTE")` moves the cursor past PLTE, so `cp_find("tRNS")` cannot see the earlier chunk and `trns` stays NULL. The test asserts the pixels equal a **tRNS-free** reference decode | `row_59_trns_before_plte` | [x] |
| 60 | `load_png_mem` | tRNS **after** the IDATs → `first` jumps past them, the IDAT scan finds nothing, `datalen == 0` ⇒ `"corrupt zlib structure in DEFLATE stream"`. Same for PLTE after the IDATs | `row_60_trns_after_idat` | [x] |
| 61 | `load_png_mem` | IEND present/absent x 0/1/4/37/1024 trailing bytes x all colour types; and `png_length` 0/1/7/64/4096 bytes larger than the real file | `row_61_trailing_and_iend` | [x] |
| 62 | `load_png_mem` | zlib `CMF` with `CINFO` 0..7 (`0x08,0x18,…,0x78`) — all accepted | `row_62_63_zlib_header` | [x] |
| 63 | `load_png_mem` | zlib `FLG`: every `FLEVEL` x `FCHECK ∈ {0,1,0x1F}` (FCHECK is never validated), `FDICT` clear | `row_62_63_zlib_header` | [x] |
| 64 | `load_png_mem` | IDAT payload = a **stored** DEFLATE block | `row_64_66_deflate_flavours` | [x] |
| 65 | `load_png_mem` | IDAT payload = a **fixed** DEFLATE block | `row_64_66_deflate_flavours` | [x] |
| 66 | `load_png_mem` | IDAT payload = our dynamic encoder (RLE on/off) and flate2 levels 0..9, x all colour types | `row_64_66_deflate_flavours` | [x] |
| 67 | `load_png_mem` | **garbage chunk CRCs and a garbage adler32** — never validated by the C code | `row_67_no_checksum_validation` | [x] |
| 68 | `load_png_mem` | IHDR payload longer than 13 bytes (`+0,1,2,7,64,255`), still `>= minlen` | `row_68_long_ihdr` | [x] |

### `load_png_mem` — chunk-length pointer arithmetic (`tests/chunks.rs`)

`cp_chunk` advances with a **signed** `int offset = len + 12` (sign extended, so
it can move *backwards*), while `cp_find` advances with an **unsigned**
`png->p += len + 12` (zero extended, always forwards).  Getting that asymmetry
wrong is invisible on well formed files, so it is pinned separately.  All of
these rows are compared with the paired fork driver, and the measured outcomes
include normal rejections, `SIGABRT`, `SIGSEGV` **and a genuine infinite loop** —
all reproduced identically by the Rust translation.

| # | entry point(s) | configuration | test | [x] |
|---|----------------|---------------|------|-----|
| 71 | `load_png_mem` → `cp_chunk` | IHDR declared length ∈ {0, 1, 12, 13, 0xFFFF, 0x7FFFFFF3, 0x7FFFFFF4 (first negative `offset`), 0x7FFFFFF5, 0x7FFFFFFF, 0x80000000, 0xC0000000, 0xFFFFFFF3 (`offset == -1`), 0xFFFFFFF4 (`offset == 0`), 0xFFFFFFF5, 0xFFFFFFF8, 0xFFFFFFFF} x `png_length` ∈ {full, full-1, 33, 0, -1} | `row_71_ihdr_declared_length` | [x] |
| 72 | `load_png_mem` → `cp_find` | the same 16 declared lengths on a **PLTE** chunk (unsigned advance), colour types 3 and 6 | `row_72_plte_declared_length` | [x] |
| 73 | `load_png_mem` → `cp_find` | the same 16 declared lengths on a **tRNS** chunk, with `trns_len` lying about the chunk body so `trns[index]` reads past it | `row_73_trns_declared_length` | [x] |
| 74 | `load_png_mem` → `cp_find` + `cp_chunk` | the IDAT collection loop with 1/2/3 IDATs where the **last** one carries each of the 16 declared lengths. Measured: `len = 0x0C, n = 1` → `SIGABRT`; `len >= 0x7FFFFFF4, n >= 2` → `SIGSEGV` (cursor walks ~2 GiB backwards); **`len = 0xFFFFFFF4` → the loop never terminates** (`len + 12` wraps to 0, so `cp_chunk` keeps returning the same chunk) | `row_74_idat_declared_length` | [x] |
| 75 | `load_png_mem` | several IDATs with declared lengths that overflow `datalen` (`int`): `2x0x40000000`, `2x0x7FFFFFFF`, `3x0x30000000`, `4x0x20000000`, `2x0x80000000`, `8x0x10000000` — `malloc(datalen)` may return NULL while the copy loop still runs (`memcpy(NULL + offset, …)`) | `row_75_datalen_overflow` | [x] |
| 76 | `load_png_mem` | 250 randomised chunk tables: random names (`IHDR`, `PLTE`, `tRNS`, `IDAT`, `IEND`, `gAMA`, `idat`, `\0\0\0\0`), random declared lengths, random CRCs, random ordering, random `png_length` (including negative and oversized) | `row_76_random_chunk_tables` | [x] |

### Exported data symbols

| # | entry point(s) | configuration | test | [x] |
|---|----------------|---------------|------|-----|
| 69 | `cp_fixed_table`, `cp_permutation_order`, `cp_len_extra_bits`, `cp_len_base`, `cp_dist_extra_bits`, `cp_dist_base` | read **all** bytes of all six tables from both `.so`s via `dlsym` and compare (320 / 19 / 31 / 124 / 32 / 128 bytes). `nm -S` also shows identical symbol *sizes* | `data_table_contents`, `symbol_parity` | [x] |
| 70 | `cp_error_reason` | `NULL` before the first call; set to the identical string after a failing call; **left untouched** by a successful call (asserted) | `error_reason_lifecycle` | [x] |

## Additional C quirks confirmed by the differential tests

1. `cp_stored`'s `bits_left / 8 <= LEN` check is written backwards, so **every**
   multi-stored-block stream is rejected — including anything flate2 produces at
   level 0 for input larger than 64 KiB (`ERRORS.md` row 26).
2. Chunk CRCs and the zlib adler32 are never checked.
3. A declared IHDR width of 0 yields `w == 1` and therefore `img.w == 0`: a
   zero-pixel-wide image that the C code happily accepts.
4. On the error path `img.w` / `img.h` retain the values reached before the
   failure; only `pix` is reset to `NULL`.
5. `cp_convert` / `cp_depalette` write `w*h` pixels, so exactly `img.w*img.h*4`
   bytes of the returned buffer are defined; the tail of the `(img.w+1)*img.h*4`
   allocation holds leftover unfiltered bytes (and, for `w < 3` with `bpp == 1`,
   never-written `malloc` memory). The tests compare exactly `img.w*img.h*4`.
6. Only bit depth 8 is supported, and interlacing (even the legal Adam7 value 1)
   is rejected.
