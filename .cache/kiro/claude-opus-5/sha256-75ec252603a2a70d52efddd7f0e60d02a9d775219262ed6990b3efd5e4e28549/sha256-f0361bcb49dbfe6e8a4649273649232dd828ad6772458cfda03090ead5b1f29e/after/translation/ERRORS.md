# ERRORS.md — error-surface table

Derived mechanically from `c_src/src/lib.c` by grepping every
`cp_error_reason = ...`, every `return 0;` in a `static int` helper, every
`return 0`/`return NULL` sentinel in a pointer-returning helper, every
`assert(...)`, and every explicit range/magic-value check. Line numbers refer
to `c_src/src/lib.c`.

Observable result for a rejection is the pair
`(return value, cp_error_reason string)` — `cp_error_reason` is an exported
symbol so the exact message is compared, not merely "both failed".

`load_png_mem` rejections all return `cp_image_t { w, h, pix = NULL }`. Note
the C sets `img.w`/`img.h` *before* several of the later checks, so the
returned `w`/`h` are **not** zero on those paths; the tests compare all three
fields.

## A. `cp_inflate` / DEFLATE layer

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| A1 | `cp_stored` (L162) | stored block (btype=0) whose `LEN != (uint16_t)~NLEN` | `cp_stored`→0, `cp_inflate`→0, `cp_error_reason = "Failed to find LEN and NLEN as complements within stored (uncompressed) stream."` |
| A2 | `cp_stored` (L171) | stored block where `s->bits_left / 8 > (int)LEN` (i.e. **more** input remains than LEN — note the check's unusual direction) | `cp_inflate`→0, `"Stored block extends beyond end of input stream."` |
| A3 | `cp_block` (L235) | literal symbol decoded while `s->out + 1 > s->out_end` (out buffer full) | `cp_inflate`→0, `"Attempted to overwrite out buffer while outputting a symbol."` |
| A4 | `cp_block` (L265) | length/distance pair with `s->out - backwards_distance < s->begin` | `cp_inflate`→0, `"Attempted to write before out buffer (invalid backwards distance)."` |
| A5 | `cp_block` (L272) | length/distance pair with `s->out + length > s->out_end` | `cp_inflate`→0, `"Attempted to overwrite out buffer while outputting a string."` |
| A6 | `cp_inflate` (L348) | `btype == 3` in a block header | `cp_inflate`→0, `"Detected unknown block type within input stream."` |
| A7 | `cp_inflate` | `in_bytes == 0` → `bits_left == 0` → `assert(s->bits_left > 0)` in first `cp_read_bits` | SIGABRT (assertion failure) |
| A8 | `cp_read_bits` (L110) | any read attempted once `bits_left <= 0` (stream exhausted mid-block) | SIGABRT |
| A9 | `cp_read_bits` (L112) | `cp_would_overflow`: `bits_left + count - num_bits < 0` | SIGABRT |
| A10 | `cp_consume_bits` (L100) | `s->count < num_bits_to_read` | SIGABRT |
| A11 | `cp_peak_bits` (L89) | `word_index > word_count` | SIGABRT (unreachable in practice) |
| A12 | `cp_ptr` (L80) | `cp_stored` reaching `cp_ptr` with `bits_left & 7 != 0` | SIGABRT |
| A13 | `cp_build` (L139) | a code length `>= 16` in the table (only reachable with a corrupt dynamic header, `lens` values come from 3-bit reads so `<= 7`; reachable via run-length overflow) | SIGABRT |
| A14 | `cp_decode` (L202) | binary search lands on a key whose prefix does not match (`lo == 0` → `tree[-1]`, or an incomplete Huffman tree) | SIGABRT |
| A15 | `cp_inflate` | `out_bytes == 0` with any literal in the stream | → A3 |
| A16 | `cp_inflate` | `in_bytes < 0` → `bits_left` negative → `assert(bits_left > 0)` | SIGABRT |
| A17 | `cp_inflate` | `in = NULL` (with `in_bytes > 0`) | SIGSEGV on first word read |

## B. `cp_unfilter`

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| B1 | `cp_unfilter` (L425, row 0) | filter byte of scanline 0 is `> 4` | →0, caller sets `"invalid filter byte found"` |
| B2 | `cp_unfilter` (L459, rows ≥1) | filter byte of any scanline `y >= 1` is `> 4` | →0, caller sets `"invalid filter byte found"` |

## C. Chunk walkers (sentinel `0` / `NULL`)

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| C1 | `cp_chunk` (L388) | 4 bytes at `p+4` != requested chunk name | `NULL` |
| C2 | `cp_chunk` (L388) | `len < minlen` | `NULL` |
| C3 | `cp_chunk` (L388) | `png->p + (int)(len + 12) > png->end` | `NULL` (and `png->p` unchanged) |
| C4 | `cp_find` (L400) | no matching chunk before `png->end` | `NULL` (and `png->p` left `>= end`) |

## D. `load_png_mem`

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| D1 | L529 | first 8 bytes != `"\211PNG\r\n\032\n"` | `pix = NULL`, `"incorrect file signature (is this a png file?)"` |
| D2 | L539 | `cp_chunk(&png,"IHDR",13)` returns NULL: name at offset 12 isn't `IHDR`, **or** `len < 13`, **or** the chunk runs past `end` (i.e. truncated file) | `pix = NULL`, `"unable to find IHDR chunk"` |
| D3 | L549 | `ihdr[8] != 8` (bit depth 1/2/4/16/0/anything) | `pix = NULL`, `"only bit-depth of 8 is supported"` |
| D4 | L574 | `ihdr[9]` (color type) ∉ {0,2,3,4,6} — includes 1,5,7 and 8..255 | `pix = NULL`, `"unknown color type"` |
| D5 | L585 | `w = make32(ihdr)+1 < 1`, i.e. raw width `== 0xFFFFFFFF` or `>= 0x80000000` | `pix = NULL`, `"invalid IHDR chunk found, image width was less than 1"` |
| D6 | L593 | `h = make32(ihdr+4) < 1`, i.e. raw height `0` or `>= 0x80000000` | `pix = NULL`, `"invalid IHDR chunk found, image height was less than 1"` |
| D7 | L602 | `!((int64_t)w*h*sizeof(cp_pixel_t) < INT_MAX)` — note `sizeof` makes the product **unsigned 64-bit**, so a negative `(int64_t)w*h` wraps to a huge value and also trips this | `pix = NULL`, `"image too large"` |
| D8 | L614 | `malloc(pix_bytes)` returns NULL | `pix = NULL`, `"unable to allocate raw image space"` (practically unreachable; `pix_bytes < INT_MAX`) |
| D9 | L625 | `ihdr[10] != 0` (compression method) | `pix = NULL`, `"only standard compression DEFLATE is supported"` |
| D10 | L633 | `ihdr[11] != 0` (filter method) | `pix = NULL`, `"only standard adaptive filtering is supported"` |
| D11 | L641 | `ihdr[12] != 0` (interlace) | `pix = NULL`, `"interlacing is not supported"` |
| D12 | L675 | `!(data && datalen >= 6)`: no IDAT chunk at all, or total IDAT payload `< 6` bytes | `pix = NULL`, `"corrupt zlib structure in DEFLATE stream"` |
| D13 | L683 | `(data[0] & 0x0f) != 0x08` (zlib CM field) | `pix = NULL`, `"only zlib compression method (RFC 1950) is supported"` |
| D14 | L691 | `(data[0] & 0xf0) > 0x70` (CINFO > 7, window too large) | `pix = NULL`, `"innapropriate window size detected"` |
| D15 | L699 | `data[1] & 0x20` (FDICT set) | `pix = NULL`, `"preset dictionary is present and not supported"` |
| D16 | L707 | `cp_out_size(&img,4) = (w+1)*h*4 < 1` (integer overflow of the product) | `pix = NULL`, `"invalid image size found"` |
| D17 | L715 | `cp_out_size(&img,bpp) = (w+1)*h*bpp < 1` | `pix = NULL`, `"invalid image size found"` |
| D18 | L724 | `cp_inflate` returns 0 for any reason in A1..A6 | `pix = NULL`, `"DEFLATE algorithm failed"` (overwrites the inner reason) |
| D19 | L732 | `cp_unfilter` returns 0 (B1/B2) | `pix = NULL`, `"invalid filter byte found"` |
| D20 | L741 | `color_type == 3` but no `PLTE` chunk found | `pix = NULL`, `"color type of indexed requires a PLTE chunk"` |
| D21 | L525 | `png_data == NULL` / `png_length == 0` — the signature `memcmp` reads 8 bytes with **no** length check first | out-of-bounds read; for a short-but-valid allocation it is a normal D1 rejection |

## E. Generic FFI boundary cases (covered even though not table rows)

| # | entry point | trigger | expected |
|---|-------------|---------|----------|
| E1 | `load_png_mem` | `png_length = 0` on a buffer that still holds ≥8 readable bytes of non-signature data | D1 |
| E2 | `load_png_mem` | `png_length` negative | `png.end < png.p`; `cp_chunk` for IHDR fails → D2 |
| E3 | `load_png_mem` | `png_length` huge (oversized, larger than the real buffer) | walks off the buffer — both must agree on whatever they read from the *same* backing allocation |
| E4 | `load_png_mem` | valid 8-byte signature then nothing (`png_length = 8`) | D2 |
| E5 | `load_png_mem` | colour type one past valid (`1`, `5`, `7`) and out-of-enum (`8`, `127`, `255`) — C enums/`switch` accept any `int` | D4 |
| E6 | `load_png_mem` | bit depth one step past valid (`7`, `9`) and all of `0,1,2,4,16` | D3 |
| E7 | `load_png_mem` | filter byte one past valid (`5`) and `255` | D19 |
| E8 | `cp_inflate` | `out_bytes = 0`, `out_bytes` negative | A3 / A5 |
| E9 | `cp_inflate` | `in_bytes` one past the real buffer length | reads past — both must agree |
| E10 | `cp_inflate` | `out = NULL` with `out_bytes = 0` and an empty final stored block | both succeed identically |

---

# Row status

All differential tests load **both** `.so`s with `libloading` and compare the
return sentinel *and* the exact `cp_error_reason` string read from each
library's own exported global. `[x]` = the row's test passes against both.

## Reachable rows

| row | test | [x] |
|-----|------|-----|
| A1 | `phase_c_errors::a1_stored_len_nlen_not_complements` | [x] |
| A2 | `phase_c_errors::a2_stored_block_extends_beyond_input` (4 shapes) | [x] |
| A3 | `phase_c_errors::a3_literal_overflows_out_buffer` (`out_bytes` 0/1/3/7) | [x] |
| A4 | `phase_c_errors::a4_backwards_distance_before_out_buffer` (5 distances) | [x] |
| A5 | `phase_c_errors::a5_string_overflows_out_buffer` (4 shapes) | [x] |
| A6 | `phase_c_errors::a6_unknown_block_type`, `a6b_non_final_unknown_block_type` | [x] |
| A7 | `isolated` "A7 in_bytes=0" — C aborts in `cp_read_bits`, verified via stderr | [x] |
| A8 | `isolated` "A8 truncated by N" (5 truncations) | [x] |
| A9 | `isolated` "A9/A10 stored header tail=N" (8 tails) | [x] |
| A10 | same as A9 — the C's assertion text confirms `cp_consume_bits` | [x] |
| A14 | `isolated` "A14 fuzz fixed body" (400) + "A13/A14 fuzz dynamic header" (600); `cp_decode`'s assertion is confirmed reached via the C's stderr | [x] |
| A15 | `phase_c_errors::a3_...` with `out_bytes = 0` | [x] |
| A16 | `isolated` "A16 in_bytes=N" for `-1, -4, -1000, i32::MIN` | [x] |
| A17 | `isolated` "A17 in=NULL in_bytes=N" — both SIGSEGV | [x] |
| B1 | `phase_c_errors::b1_row0_filter_byte_out_of_range` (5 bytes × 5 colour types) | [x] |
| B2 | `phase_c_errors::b2_later_row_filter_byte_out_of_range` (4 bytes × 3 rows) | [x] |
| C1 | `phase_c_errors::c1_chunk_name_mismatch` | [x] |
| C2 | `phase_c_errors::c2_chunk_len_below_minlen` (declared 0/1/12) | [x] |
| C3 | `phase_c_errors::c3_chunk_extends_past_end` (12 truncations + over-declared) | [x] |
| C4 | `phase_c_errors::c4_find_finds_nothing` | [x] |
| D1 | `phase_c_errors::d1_bad_signature` (+ every single-byte signature corruption) | [x] |
| D2 | `phase_c_errors::d2_missing_ihdr` | [x] |
| D3 | `phase_c_errors::d3_unsupported_bit_depth` (0,1,2,4,7,9,16,32,255) | [x] |
| D4 | `phase_c_errors::d4_unknown_color_type` (1,5,7,8,9,16,100,127,128,200,254,255) | [x] |
| D5 | `phase_c_errors::d5_width_less_than_one` (5 raw widths) | [x] |
| D6 | `phase_c_errors::d6_height_less_than_one` (4 raw heights) | [x] |
| D7 | `phase_c_errors::d7_image_too_large` (5 shapes) | [x] |
| D9 | `phase_c_errors::d9_bad_compression_method` | [x] |
| D10 | `phase_c_errors::d10_bad_filter_method` | [x] |
| D11 | `phase_c_errors::d11_interlace_unsupported` | [x] |
| D12 | `phase_c_errors::d12_corrupt_zlib_structure` (no IDAT, len 0..5, split) | [x] |
| D13 | `phase_c_errors::d13_bad_zlib_compression_method` (all 15 wrong CM values) | [x] |
| D14 | `phase_c_errors::d14_window_size_too_large` (CINFO 8..15) | [x] |
| D15 | `phase_c_errors::d15_preset_dictionary` (5 FLG values) | [x] |
| D18 | `phase_c_errors::d18_deflate_failure_is_reported` (4 distinct inner failures) | [x] |
| D19 | B1 / B2 above | [x] |
| D20 | `phase_c_errors::d20_indexed_without_plte` | [x] |
| D21 | `phase_c_errors::d21_oversized_and_undersized_lengths`; `isolated` "D21b" (4 bit-flips × every byte, every truncation, 5 over-declared lengths) | [x] |
| E1–E8 | `phase_c_errors::d1_...`, `d2_...`, `d4_...`, `e7_filter_byte_one_past_valid`, `a3_...` | [x] |
| E9 | `isolated` "E9 in_bytes+N" | [x] |
| E10 | `phase_c_errors::e10_null_out_with_empty_stored_block` | [x] |

## Rows that are unreachable, with the evidence

These are **not** skipped: each is unreachable by construction, and the claim is
backed by a test that probes the surrounding region and shows the neighbouring
branch fires instead — in both implementations.

| row | why unreachable | evidence |
|-----|-----------------|----------|
| A11 | `cp_peak_bits` only increments `word_index` inside `if (word_index < word_count)`, so `word_index <= word_count` is an invariant | assertion never fires across 3694 `isolated` cases; the C's stderr shows only `cp_read_bits`, `cp_consume_bits` and `cp_decode` ever assert |
| A12 | `bits_left ≡ count (mod 8)` holds from entry (both start as multiples of 8) and `cp_consume_bits` changes them equally; `cp_stored` aligns via `cp_read_bits(s, s->count & 7)`, which therefore also aligns `bits_left` | `isolated` "A12 sweep" — 28 block-1 lengths × 4 input alignments × 14 stored lengths = 1568 systematic attempts to desynchronise the two counters; `cp_ptr`'s assertion never fires |
| A13 | every `lens[]` value fed to `cp_build` is ≤ 15 by construction: `lenlens` comes from 3-bit reads (≤ 7), `cp_fixed_table` holds 5..9, and the `cp_dynamic` loop writes either a 3-bit-derived value, a copy of one, or 0 | `cp_build`'s assertion never fires across 3694 cases, including 600 fuzzed dynamic headers and 71 deliberate `lens[320]` overruns |
| D8 | D7 guarantees `1 <= (int64)w*h*4 < INT_MAX`, so `malloc(pix_bytes)` asks for < 2 GiB and does not fail here | `phase_c_errors::d8_d16_d17_boundary_sweep` sweeps 7 heights × 5 offsets straddling the exact `INT_MAX/4` boundary and asserts the message is never "unable to allocate raw image space" |
| D16, D17 | `cp_out_size(img,bpp) == w*h*bpp` with `bpp <= 4` is the same product D7 already bounded, and `w,h >= 1` makes it `>= 1` | same sweep asserts the message is never "invalid image size found" |

## Note on build profile

The artifact under verification is the **release** cdylib (`crate-type =
["cdylib"]`, `[profile.release] panic = "abort"`). Under `RUST_SO` pointing at
the *debug* cdylib, the three `A17` (`in == NULL`) cases diverge: the C
segfaults while Rust aborts, because rustc's `debug_assertions` insert a
"null pointer dereference occurred" check that fires before the faulting load.
That is a property of the debug profile's instrumentation, not of the
translation; every other case matches in both profiles.
