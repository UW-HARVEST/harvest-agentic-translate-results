# ERRORS.md — error-surface table

Derived mechanically from `c_src/src/lib.c`. Two kinds of rejection exist:

* **graceful** — `cp_error_reason = "…"; goto cp_err;` (a `RETURN_ERROR`-style
  macro that has been expanded in the source) ⇒ `cp_inflate` returns `0` /
  `load_png_mem` returns `{w, h, pix = NULL}` with `cp_error_reason` set.
* **abort** — a live `assert()` (the reference `.so` is built without `NDEBUG`,
  see `SYMBOLS.md`) ⇒ `__assert_fail` ⇒ `SIGABRT` (signal 6).

`cp_error_reason` is *not* reset on success, so its value only means anything
after a failure.

## Graceful rejections

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `cp_stored` | `LEN != (uint16_t)~NLEN` | `0`, reason `"Failed to find LEN and NLEN as complements within stored (uncompressed) stream."` → `cp_inflate` 0 |
| 2 | `cp_stored` | `!(s->bits_left / 8 <= (int)LEN)` (i.e. more than `LEN` bytes still left in the stream) | `0`, reason `"Stored block extends beyond end of input stream."` |
| 3 | `cp_block` | literal symbol (`< 256`) and `!(s->out + 1 <= s->out_end)` | `0`, reason `"Attempted to overwrite out buffer while outputting a symbol."` |
| 4 | `cp_block` | back-reference and `!(s->out - backwards_distance >= s->begin)` | `0`, reason `"Attempted to write before out buffer (invalid backwards distance)."` |
| 5 | `cp_block` | back-reference and `!(s->out + length <= s->out_end)` | `0`, reason `"Attempted to overwrite out buffer while outputting a string."` |
| 6 | `cp_inflate` | `btype == 3` | `0`, reason `"Detected unknown block type within input stream."` |
| 7 | `load_png_mem` | `memcmp(png_data, "\x89PNG\r\n\x1a\n", 8) != 0` | `pix = NULL`, `"incorrect file signature (is this a png file?)"` |
| 8 | `load_png_mem` | `cp_chunk(&png,"IHDR",13) == NULL` — first chunk is not `IHDR`, or its length `< 13`, or `p + len + 12 > end` | `pix = NULL`, `"unable to find IHDR chunk"` |
| 9 | `load_png_mem` | `ihdr[8] != 8` (bit depth) | `pix = NULL`, `"only bit-depth of 8 is supported"` |
| 10 | `load_png_mem` | `ihdr[9] ∉ {0,2,3,4,6}` (colour type) | `pix = NULL`, `"unknown color type"` |
| 11 | `load_png_mem` | `w = cp_make32(ihdr)+1 < 1` (as `int`; e.g. width `0x7FFFFFFF` → `w = INT_MIN`, or width `0xFFFFFFFF` → `w = 0`) | `pix = NULL`, `"invalid IHDR chunk found, image width was less than 1"` |
| 12 | `load_png_mem` | `h = cp_make32(ihdr+4) < 1` (as `int`; `0` or any value `≥ 0x80000000`) | `pix = NULL`, `"invalid IHDR chunk found, image height was less than 1"` |
| 13 | `load_png_mem` | `!((uint64_t)((int64_t)w*h) * 4 < INT_MAX)` | `pix = NULL`, `"image too large"` |
| 14 | `load_png_mem` | `malloc(pix_bytes)` returns `NULL` | `pix = NULL`, `"unable to allocate raw image space"` (unreachable in practice: `pix_bytes < INT_MAX`) |
| 15 | `load_png_mem` | `ihdr[10] != 0` (compression method) | `pix = NULL`, `"only standard compression DEFLATE is supported"` |
| 16 | `load_png_mem` | `ihdr[11] != 0` (filter method) | `pix = NULL`, `"only standard adaptive filtering is supported"` |
| 17 | `load_png_mem` | `ihdr[12] != 0` (interlace) | `pix = NULL`, `"interlacing is not supported"` |
| 18 | `load_png_mem` | `!(data && datalen >= 6)` — no `IDAT` at all (`datalen == 0` ⇒ `malloc(0)`, which glibc makes non-NULL, so the `datalen >= 6` half is what fires), or total IDAT payload `< 6`, or `datalen < 0` after `uint32` wrap ⇒ `malloc` fails ⇒ `data == NULL` | `pix = NULL`, `"corrupt zlib structure in DEFLATE stream"` |
| 19 | `load_png_mem` | `(data[0] & 0x0f) != 0x08` (zlib CM) | `pix = NULL`, `"only zlib compression method (RFC 1950) is supported"` |
| 20 | `load_png_mem` | `(data[0] & 0xf0) > 0x70` (zlib CINFO > 7) | `pix = NULL`, `"innapropriate window size detected"` (sic) |
| 21 | `load_png_mem` | `data[1] & 0x20` (FDICT set) | `pix = NULL`, `"preset dictionary is present and not supported"` |
| 22 | `load_png_mem` | `!(cp_out_size(&img,4) >= 1)`, i.e. `(int)(w*h*4) < 1` | `pix = NULL`, `"invalid image size found"` (unreachable: check #13 already bounds `w*h*4 < INT_MAX` and `w,h ≥ 1`) |
| 23 | `load_png_mem` | `!(cp_out_size(&img,bpp) >= 1)` | `pix = NULL`, `"invalid image size found"` (unreachable, same reason) |
| 24 | `load_png_mem` | `cp_inflate(...) == 0` (any of #1–#6) | `pix = NULL`, `"DEFLATE algorithm failed"` — note `cp_error_reason` is **overwritten**, the inflate reason is lost |
| 25 | `load_png_mem` | `cp_unfilter` sees a filter byte `> 4` on row 0 | `pix = NULL`, `"invalid filter byte found"` |
| 26 | `load_png_mem` | `cp_unfilter` sees a filter byte `> 4` on any row `y ≥ 1` | `pix = NULL`, `"invalid filter byte found"` |
| 27 | `load_png_mem` | `color_type == 3` and no `PLTE` chunk was found | `pix = NULL`, `"color type of indexed requires a PLTE chunk"` |

## Aborting rejections (live `assert`s)

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| A1 | `cp_ptr` (from `cp_stored`) | `s->bits_left & 7` — stream not byte aligned after LEN/NLEN | `SIGABRT` |
| A2 | `cp_peak_bits` | `s->word_index > s->word_count` after the increment — unreachable (guarded by `word_index < word_count`) | `SIGABRT` |
| A3 | `cp_consume_bits` | `s->count < num_bits_to_read` — asking for more bits than are buffered (input exhausted mid-symbol) | `SIGABRT` |
| A4 | `cp_read_bits` | `num_bits_to_read > 32` — only from `cp_block` via a mutated `cp_len_extra_bits`/`cp_dist_extra_bits`, or an out-of-range table read landing on a byte `> 32` | `SIGABRT` |
| A5 | `cp_read_bits` | `num_bits_to_read < 0` — unreachable (all call sites pass literals or `uint8_t` table entries) | `SIGABRT` |
| A6 | `cp_read_bits` | `!(s->bits_left > 0)` — the input stream is exhausted. **`cp_inflate(in, 0, …)` hits this on the very first `cp_read_bits(s,1)`**, as does `load_png_mem` on a PNG whose IDAT payload is exactly 6 bytes | `SIGABRT` |
| A7 | `cp_read_bits` | `!(s->count <= 64)` | `SIGABRT` |
| A8 | `cp_read_bits` | `cp_would_overflow(s,n)`, i.e. `(bits_left + count) - n < 0` | `SIGABRT` |
| A9 | `cp_build` | any `lens[i] != 0 && lens[i] >= 16` for `i < sym_count` (reachable: `cp_dynamic` stores raw decoded symbols ≥ 19 into `lens`, and a caller may mutate `cp_fixed_table`) | `SIGABRT` |
| A10 | `cp_decode` | `(search >> (32 - (key & 0xF))) != (key >> (32 - (key & 0xF)))` — the buffered bits do not match the Huffman entry found. Fires for an incomplete/empty tree (including the `hi == 0` case that reads `tree[-1]`) and for over-subscribed trees. Shifts are 32-bit `shr %cl`, so `key & 0xF == 0` ⇒ shift count 0 ⇒ the test degenerates to `search == key` | `SIGABRT` |

## Generic FFI boundary cases (not distinct C checks, tested anyway)

| # | entry point | input | expected C result |
|---|-------------|-------|-------------------|
| G1 | `load_png_mem` | `png_length == 0` with a valid-signature buffer | `memcmp` still reads 8 bytes; `png.end = p`; behaves per the buffer contents (usually #8) |
| G2 | `load_png_mem` | `png_length < 0` | `png.end < png.p`, `cp_find` loop never runs ⇒ #18 |
| G3 | `load_png_mem` | truncated file, 1..7 bytes of signature | #7 or #8 depending on bytes read past the buffer (C reads out of bounds — only tested with an over-allocated buffer so both libraries read the same bytes) |
| G4 | `cp_inflate` | `in_bytes == 0` | A6 (`SIGABRT`) |
| G5 | `cp_inflate` | `in_bytes < 0` | `bits_left < 0` ⇒ A6/A8 (`SIGABRT`) |
| G6 | `cp_inflate` | `out_bytes == 0` | `out_end == out` ⇒ #3 or #5 on the first output byte |
| G7 | `cp_inflate` | `out_bytes < 0` | `out_end < begin` ⇒ #3 |
| G8 | `load_png_mem` | colour type `1`, `5`, `7`, `255` (values with no valid `switch` case) | #10 — this is the "out-of-range enum across FFI" case |
| G9 | `load_png_mem` | filter byte `5..255` in the inflated data | #25 / #26 |
| G10 | `cp_inflate` | `btype` is 2 bits, so all 4 values are valid inputs; `3` is #6 | see #6 |


## Status

| row | test | result |
|---|---|---|
| 1-6 | `phase_c_errors::rows_01_06_inflate_graceful` | pass (error string + `cp_inflate == 0` asserted) |
| 7-13, 15-21, 27 | `phase_c_errors::rows_07_21_27_png_structural` | pass (error string + `pix == NULL` asserted) |
| 24-26 | `phase_c_errors::rows_24_26_deflate_and_filter_failures` | pass |
| 14, 22, 23 | `phase_c_errors::generic_boundaries` | unreachable by the C's own arithmetic (see below); the closest reachable inputs are compared |
| A1, A2, A5 | `phase_c_errors::generic_boundaries` | unreachable (see below); the closest reachable inputs are compared |
| A3, A4, A6, A9, A10 | `phase_c_errors::rows_a3_a10_aborts` | pass (`SIGABRT` asserted) |
| A7, A8 | `fuzz::fuzz_cp_inflate_with_table_mutations` | pass (reached by a retuned extra-bits table on a nearly-exhausted stream) |
| G1-G10 | `rows_07_21_27_png_structural`, `rows_01_06_inflate_graceful`, `generic_boundaries`, `null_pointer` | pass |

### Why rows 14, 22, 23, A1, A2 and A5 are unreachable

* **14** — `malloc(pix_bytes)` with `pix_bytes < INT_MAX` (guaranteed by check
  13) never returns `NULL` on a 64-bit host with overcommit.
* **22, 23** — check 13 already establishes `(int64)w*h*4 < INT_MAX` with
  `w, h >= 1`, so `cp_out_size(&img, 4)` and `cp_out_size(&img, bpp)` are both
  `>= 1`; the two "invalid image size found" branches are dead.
* **A1** — `bits_left ≡ count (mod 8)` is invariant while `loaded ≡ 0 (mod 8)`,
  and the only load that can break that (the `final_word` path, which adds
  `bits_left` rather than `last_bytes*8`) can only fire when `count_then ≡ 0`:
  `cp_read_bits(s,1)` always leaves `count >= 7`, so `cp_read_bits(s,2)` never
  triggers a load, and after `cp_stored` discards `count & 7` bits `bits_left` is
  a multiple of 8 and stays one. Hence `assert(!(s->bits_left & 7))` in `cp_ptr`
  cannot fire.
* **A2** — `assert(s->word_index <= s->word_count)` sits directly under
  `if (s->word_index < s->word_count)`, so it is true by construction.
* **A5** — every `cp_read_bits` call site passes either a literal (1, 2, 3, 4, 5,
  7, 16) or a `uint8_t` table entry, so `num_bits_to_read` is never negative.
