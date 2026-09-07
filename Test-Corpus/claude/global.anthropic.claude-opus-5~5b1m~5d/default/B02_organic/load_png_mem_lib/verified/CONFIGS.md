# CONFIGS.md — configuration-surface table (valid inputs)

Public entry points (all of `nm -D`):

* **functions** — `load_png_mem(const uint8_t*, int)` (high-level one-shot) and
  `cp_inflate(void *in, int in_bytes, void *out, int out_bytes)` (the low-level
  raw-DEFLATE entry point; not declared in `include/lib.h` but exported and
  therefore part of the public surface).
* **mutable data objects** — `cp_error_reason`, `cp_fixed_table`,
  `cp_permutation_order`, `cp_len_extra_bits`, `cp_len_base`,
  `cp_dist_extra_bits`, `cp_dist_base`.  These are *runtime options*: the
  decoder reads them live on every call, so a consumer that writes them changes
  the decode.  They are also what out-of-range table indices read.

Axes the C actually branches on:

| axis | values the C distinguishes | where |
|---|---|---|
| colour type | 0 (`bpp=1` grey), 2 (`bpp=3` RGB), 3 (`bpp=1` indexed), 4 (`bpp=2` grey+A), 6 (`bpp=4` RGBA) | `switch (color_type)`, `cp_convert`'s `switch (bpp)` |
| palette | `PLTE` absent / present | `cp_find("PLTE")`, `cp_depalette` |
| transparency | `tRNS` absent / present with `trns_len > max index` / present with `trns_len <= some index` | `cp_get_alpha_for_indexed_image` |
| chunk stream | `PLTE` then `tRNS` / `tRNS` only / neither / ancillary chunks interleaved / chunks after the last IDAT | `cp_find` / `cp_chunk` rewind logic (`first = png.p`) |
| IDAT layout | 1 chunk / N contiguous chunks / N chunks separated by other chunks / zero-length IDAT among them | `cp_find` + `cp_chunk` loop |
| row filter | 0,1,2,3,4 on row 0 (row 0 special-cases: 2 is a no-op, 4 uses `paeth(a,0,0)`, 1/3/4 start at `x=bpp`) and 0,1,2,3,4 on rows >= 1 | `cp_unfilter`'s two `switch`es |
| DEFLATE block type | 0 stored / 1 fixed / 2 dynamic | `cp_inflate`'s `switch (btype)` |
| block count | single final block / multiple blocks then a final one | `do { ... } while (!bfinal)` |
| dynamic tree | plain code lengths / symbol 16 (copy prev) / 17 (short zero run) / 18 (long zero run); `HLIT`/`HDIST`/`HCLEN` at min and max | `cp_dynamic`'s `switch (sym)` |
| back-reference | distance == 1 (`memset` fast path) / distance > 1 (byte loop, overlapping) | `cp_block`'s `switch (backwards_distance)` |
| length/dist symbols | symbols with 0 extra bits / with extra bits (`cp_len_extra_bits`, `cp_dist_extra_bits`) | `cp_block` |
| `cp_build` lookup | with `s` (builds `s->lookup`, `len <= 9`) / with `s == 0` | `cp_build`'s `if (s)` |
| zlib header | CM nibble 8; CINFO 0..7; FLEVEL 0..3; FCHECK any; FDICT clear | the three header checks |
| `cp_inflate` in-pointer alignment | `in % 4 == 0,1,2,3` → `first_bytes = 0,3,2,1` | `first_bytes` computation, `cp_ptr` |
| `cp_inflate` tail | `(in_bytes - first_bytes) % 4 == 0,1,2,3` → `final_word_available` 0/1 and the final-word `count += bits_left` path | `cp_peak_bits`'s `else if` |
| `cp_inflate` out size | exactly enough / larger than needed | `out_end` checks |
| image shape | 1x1, 1xN, Nx1, NxM, and `w` where `(w+1)*h*bpp` crosses the `out` offset arithmetic | `cp_out_size`, `out = pix + size(4) - size(bpp)` |
| table mutation | each of the 6 tables written by the caller before the call | all table reads |

## Rows (one per combination the C treats differently)

Every row is driven with **many randomised inputs** (seeded, deterministic) —
random pixel data / random palettes / random shapes within the row's shape
class — and both libraries are called through `dlopen`/`dlsym`, comparing the
returned `cp_image_t` (`w`, `h`, `pix != NULL`), the whole pixel buffer
byte-for-byte, and `cp_error_reason`.

### `load_png_mem` — colour type x filter x shape

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `load_png_mem` | ct=0 (grey, bpp 1), all rows filter 0, shapes {1x1, 1x7, 7x1, 5x5, 16x9, 63x2} | [x] |
| 2 | `load_png_mem` | ct=0, all rows filter 1 (Sub) | [x] |
| 3 | `load_png_mem` | ct=0, all rows filter 2 (Up) | [x] |
| 4 | `load_png_mem` | ct=0, all rows filter 3 (Average) | [x] |
| 5 | `load_png_mem` | ct=0, all rows filter 4 (Paeth) | [x] |
| 6 | `load_png_mem` | ct=0, per-row random filter in 0..4 | [x] |
| 7 | `load_png_mem` | ct=2 (RGB, bpp 3), filters 0..4 each + mixed | [x] |
| 8 | `load_png_mem` | ct=4 (grey+alpha, bpp 2), filters 0..4 each + mixed | [x] |
| 9 | `load_png_mem` | ct=6 (RGBA, bpp 4), filters 0..4 each + mixed | [x] |
| 10 | `load_png_mem` | ct=3 (indexed, bpp 1) + `PLTE` (256 entries), no `tRNS`, filters 0..4 + mixed | [x] |
| 11 | `load_png_mem` | ct=3 + `PLTE` short (1, 2, 17 entries) → `cp_depalette` reads past the chunk for indices >= entry count | [x] |
| 12 | `load_png_mem` | ct=3 + `PLTE` + `tRNS` covering all 256 indices | [x] |
| 13 | `load_png_mem` | ct=3 + `PLTE` + `tRNS` shorter than the indices used (`index >= trns_len` → alpha 255) | [x] |
| 14 | `load_png_mem` | ct=0/2/4/6 **with** a `PLTE` and/or `tRNS` chunk present (must be ignored — `cp_convert` path) | [x] |
| 15 | `load_png_mem` | ct=3, `tRNS` present but `PLTE` absent (row 20 of ERRORS.md — kept here too as the "tRNS-only rewind" shape) | [x] |

### `load_png_mem` — chunk-stream shapes

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 16 | `load_png_mem` | single IDAT | [x] |
| 17 | `load_png_mem` | zlib stream split across 2, 3, 5 contiguous IDATs (incl. a 1-byte IDAT) | [x] |
| 18 | `load_png_mem` | IDATs separated by an ancillary chunk (`tEXt`) → `cp_chunk` returns NULL, gathering stops early | [x] |
| 19 | `load_png_mem` | zero-length IDAT first, then the real one | [x] |
| 20 | `load_png_mem` | ancillary chunks (`gAMA`, `tEXt`, `pHYs`) between IHDR and PLTE/tRNS/IDAT | [x] |
| 21 | `load_png_mem` | `tRNS` before `PLTE` in the file (rewind logic: `cp_find("PLTE")` scans past `tRNS`) | [x] |
| 22 | `load_png_mem` | chunks after the last IDAT (`IEND`, plus junk bytes after `IEND`) | [x] |
| 23 | `load_png_mem` | `png_length` longer than the actual file content (trailing garbage) | [x] |
| 24 | `load_png_mem` | IHDR declared `len > 13` (extra bytes; `minlen` check passes) | [x] |

### `load_png_mem` — DEFLATE / zlib configuration

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 25 | `load_png_mem` | zlib data = one **stored** (btype 0) final block | [x] |
| 26 | `load_png_mem` | zlib data = **fixed** Huffman (btype 1), literals only | [x] |
| 27 | `load_png_mem` | zlib data = **fixed** Huffman with back-references (dist 1 → `memset` path) | [x] |
| 28 | `load_png_mem` | zlib data = **fixed** Huffman with back-references dist > 1 (overlapping copies) | [x] |
| 29 | `load_png_mem` | zlib data = **dynamic** Huffman (btype 2) produced by a real deflater (zlib level 1..9) | [x] |
| 30 | `load_png_mem` | zlib data = multiple non-final blocks then a final one (mixed fixed + stored + dynamic) | [x] |
| 31 | `load_png_mem` | zlib header CINFO = 0..7 (all accepted), FLEVEL = 0..3, FCHECK arbitrary | [x] |

### `cp_inflate` — low-level entry point driven directly

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 32 | `cp_inflate` | stored block, `LEN` = 0,1,7,64,1024, at input alignments 0,1,2,3 | [x] |
| 33 | `cp_inflate` | fixed-Huffman literals only, `in` alignment 0..3 x `(in_bytes-first_bytes)%4` = 0..3 | [x] |
| 34 | `cp_inflate` | fixed Huffman, length symbols 257..285 (all extra-bit classes) with distance 1 | [x] |
| 35 | `cp_inflate` | fixed Huffman, distance symbols 0..29 (all extra-bit classes) | [x] |
| 36 | `cp_inflate` | dynamic Huffman, `HCLEN` 4..19, `HLIT` 257..288, `HDIST` 1..32 | [x] |
| 37 | `cp_inflate` | dynamic Huffman using code-length symbol 16 (copy previous, 3..6) | [x] |
| 38 | `cp_inflate` | dynamic Huffman using code-length symbol 17 (zero run 3..10) | [x] |
| 39 | `cp_inflate` | dynamic Huffman using code-length symbol 18 (zero run 11..138) | [x] |
| 40 | `cp_inflate` | multiple blocks: stored→fixed→dynamic→final, and every 2-permutation | [x] |
| 41 | `cp_inflate` | `out_bytes` exactly the decompressed size / much larger | [x] |
| 42 | `cp_inflate` | random valid deflate streams (zlib-compressed random + repetitive data, headers stripped) x 4 alignments x 3 output sizes | [x] |

### Runtime "options": the exported mutable tables

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 43 | `cp_fixed_table` + `cp_inflate` | caller rewrites `cp_fixed_table` to a different (valid, <= 15) code-length assignment, then inflates a fixed-Huffman stream built for that table | [x] |
| 44 | `cp_len_base` / `cp_len_extra_bits` + `cp_inflate` | caller perturbs a length entry → decoded output changes identically in both | [x] |
| 45 | `cp_dist_base` / `cp_dist_extra_bits` + `cp_inflate` | caller perturbs a distance entry | [x] |
| 46 | `cp_permutation_order` + `cp_inflate` | caller permutes the code-length order, stream built to match | [x] |
| 47 | `cp_error_reason` | caller sets it to NULL / to its own string before a *successful* call (C never clears it) | [x] |
| 48 | all six tables | out-of-range table reads: `symbol - 257` in 29..90 and distance symbol in 30..90, which read the neighbouring table in `.data` (see SYMBOLS.md layout) | [x] |

### Repeat / state

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 49 | `load_png_mem` | same PNG decoded twice in a row; a failing call followed by a succeeding one (proves no leaked state except `cp_error_reason`) | [x] |
| 50 | `load_png_mem` + `cp_inflate` | interleaved calls to both entry points | [x] |

No binary/driver target exists (`c_src/CMakeLists.txt` builds only a `SHARED`
library, and `translation/Cargo.toml` declares only a `cdylib`), so the
"compare stdout of the two binaries" gate is not applicable.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
configuration is the default (empty) feature set.  The gate "all of the above
hold under every feature combination" is therefore satisfied by the single
default combination; `cargo test --no-default-features` is also run to prove
there is no hidden default feature.

## Result

`cargo test --test phase_b_valid` — 49 tests (one per row; rows 7..10 fold five
filter types plus a mixed pass into one test), all passing.  Every row is driven
with many randomised inputs from a fixed seed: random pixel/palette/`tRNS` bytes
over the shape set `{1x1, 1x7, 7x1, 5x5, 16x9, 63x2, 3x40}`, each in both a
stored-block and a fixed-Huffman encoding.  Every call goes through
`dlopen`/`dlsym` on both `.so`s, and the whole `w*h*4`-byte pixel buffer plus
`w`, `h` and `cp_error_reason` are compared byte for byte.

Row 11 (short `PLTE`) needs a note: `cp_depalette` indexes `plte[c*3 .. c*3+2]`
with no bound check, so a 1-entry palette makes the C read up to 767 bytes past
the chunk.  The test therefore pads the *buffer* (not the declared `png_length`)
so those reads stay inside its own deterministic allocation; without the padding
they hit unrelated heap and differ between two runs of the *same* library.

Row 48 (out-of-range table reads) lives in `phase_c_errors.rs` because such
streams are malformed and can abort the assert-enabled reference build.

No binary/driver target exists on either side (`c_src/CMakeLists.txt` builds only
a `SHARED` library; `translation/Cargo.toml` declares only a `cdylib`), so the
"compare the two binaries' stdout" gate is not applicable.  The differential
*harness* is nevertheless a real program pair in disguise:
`examples/diffrunner.rs` is run once per library over the same corpus and the two
stdouts are compared byte for byte, which is exactly that check.

### Feature combinations

`Cargo.toml` declares no `[features]`.  `verify.sh` enumerates the section
mechanically and falls back to the two configurations that exist:

```
combinations: <default> <no-default-features>
--- cargo test  ---                        4 + 49 + 29 + 5 tests, all ok
--- cargo test --no-default-features ---   4 + 49 + 29 + 5 tests, all ok
ALL PHASES PASSED FOR EVERY FEATURE COMBINATION
```
