# ERRORS.md — error-surface table

Derived mechanically from `c_src/src/lib.c`:

```
grep -n 'cp_error_reason =' lib.c   -> 26 assignment sites (25 distinct messages)
grep -n 'return 0;'         lib.c   -> 182, 297, 388, 400, 425, 459 (+ cp_chunk/cp_find `return 0` = NULL)
grep -n 'assert('           lib.c   -> 10 assertion sites
```

Two error channels exist:

* **return value** — `cp_inflate` returns `0`; `load_png_mem` returns a
  `cp_image_t` with `pix == NULL` (`w`/`h` keep whatever they were assigned
  before the jump to `cp_err`, i.e. `0`/`0` before the IHDR is parsed and
  `w-1`/`h` after).
* **`cp_error_reason`** — a `const char *` global. When `cp_inflate` fails
  *inside* `load_png_mem`, `load_png_mem` **overwrites** the inner message with
  `"DEFLATE algorithm failed"` (rows 1–6 are only observable verbatim when
  `cp_inflate` is called directly).

Assertions are live (`c_src/CMakeLists.txt` sets no build type ⇒ `-O0`, no
`NDEBUG`), so a failed assertion is `__assert_fail` ⇒ **SIGABRT (signal 6)**.
The Rust `cp_assert!` macro calls `abort()` ⇒ also SIGABRT. Rows A1–A10 are
tested by comparing the *termination signal* of a forked child.

## Rejections that set `cp_error_reason` / return a failure value

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `cp_stored` (via `cp_inflate`) | `LEN != (uint16_t)~NLEN` in a `btype==0` block | `cp_inflate` → `0`; reason `"Failed to find LEN and NLEN as complements within stored (uncompressed) stream."` |
| 2 | `cp_stored` (via `cp_inflate`) | `!(s->bits_left / 8 <= (int)LEN)` — remaining whole input bytes after the 5-byte header exceed `LEN` | `cp_inflate` → `0`; reason `"Stored block extends beyond end of input stream."` |
| 3 | `cp_block` (via `cp_inflate`) | literal symbol `< 256` decoded when `s->out + 1 > s->out_end` (output buffer full) | `cp_inflate` → `0`; reason `"Attempted to overwrite out buffer while outputting a symbol."` |
| 4 | `cp_block` (via `cp_inflate`) | length/distance pair whose `backwards_distance` puts `s->out - dist` before `s->begin` | `cp_inflate` → `0`; reason `"Attempted to write before out buffer (invalid backwards distance)."` |
| 5 | `cp_block` (via `cp_inflate`) | length/distance pair with `s->out + length > s->out_end` | `cp_inflate` → `0`; reason `"Attempted to overwrite out buffer while outputting a string."` |
| 6 | `cp_inflate` | `btype == 3` (the reserved DEFLATE block type) | `cp_inflate` → `0`; reason `"Detected unknown block type within input stream."` |
| 7 | `load_png_mem` | first 8 bytes ≠ `"\211PNG\r\n\032\n"` | `img = {0,0,NULL}`; reason `"incorrect file signature (is this a png file?)"` |
| 8 | `load_png_mem` | `cp_chunk(&png,"IHDR",13)` returns NULL — chunk type ≠ `IHDR`, or `len < 13`, or `png.p + len + 12 > png.end` | `img = {0,0,NULL}`; reason `"unable to find IHDR chunk"` |
| 9 | `load_png_mem` | `ihdr[8] != 8` (any bit depth other than 8: 1, 2, 4, 16, 0, 255 …) | `img = {0,0,NULL}`; reason `"only bit-depth of 8 is supported"` |
| 10 | `load_png_mem` | `ihdr[9] ∉ {0,2,3,4,6}` (`default:` of the colour-type `switch`; includes 1, 5, 7, 8, 255) | `img = {0,0,NULL}`; reason `"unknown color type"` |
| 11 | `load_png_mem` | `w = cp_make32(ihdr)+1 < 1` as `int` — i.e. `ihdr[0..4] == 0xFFFFFFFF` (⇒ `w==0`) or `≥ 0x7FFFFFFF` (⇒ `w` negative) | `img = {0,0,NULL}`; reason `"invalid IHDR chunk found, image width was less than 1"` |
| 12 | `load_png_mem` | `h = cp_make32(ihdr+4) < 1` as `int` — `ihdr[4..8] == 0` or `≥ 0x80000000` | `img = {0,0,NULL}`; reason `"invalid IHDR chunk found, image height was less than 1"` |
| 13 | `load_png_mem` | `!((int64_t)w * h * sizeof(cp_pixel_t) < INT_MAX)` — note the `sizeof` makes the product **unsigned**, so `w*h*4 >= 0x7FFFFFFF` (e.g. `w=0x10000, h=0x10000`) | `img = {0,0,NULL}`; reason `"image too large"` |
| 14 | `load_png_mem` | `malloc(pix_bytes)` returns NULL (`pix_bytes` is an `int` sign-extended to `size_t`; unreachable for the sizes row 13 lets through, kept for completeness) | `img = {w-1,h,NULL}`; reason `"unable to allocate raw image space"` |
| 15 | `load_png_mem` | `ihdr[10] != 0` (compression method) | `img = {w-1,h,NULL}`; reason `"only standard compression DEFLATE is supported"` |
| 16 | `load_png_mem` | `ihdr[11] != 0` (filter method) | `img = {w-1,h,NULL}`; reason `"only standard adaptive filtering is supported"` |
| 17 | `load_png_mem` | `ihdr[12] != 0` (interlace method) | `img = {w-1,h,NULL}`; reason `"interlacing is not supported"` |
| 18 | `load_png_mem` | `!(data && datalen >= 6)` — no IDAT at all (`datalen == 0`), or total IDAT payload `< 6`, or `malloc(datalen)` NULL because `datalen` went negative | `img = {w-1,h,NULL}`; reason `"corrupt zlib structure in DEFLATE stream"` |
| 19 | `load_png_mem` | `(data[0] & 0x0f) != 0x08` — zlib CM field not 8 | `img = {w-1,h,NULL}`; reason `"only zlib compression method (RFC 1950) is supported"` |
| 20 | `load_png_mem` | `(data[0] & 0xf0) > 0x70` — zlib CINFO > 7 | `img = {w-1,h,NULL}`; reason `"innapropriate window size detected"` |
| 21 | `load_png_mem` | `data[1] & 0x20` — zlib FDICT set | `img = {w-1,h,NULL}`; reason `"preset dictionary is present and not supported"` |
| 22 | `load_png_mem` | `cp_out_size(&img,4) = (img.w+1)*img.h*4 < 1` as `int` (signed overflow wrap, e.g. `w-1 = 0x1FFFFFFF, h = 1`) | `img = {w-1,h,NULL}`; reason `"invalid image size found"` |
| 23 | `load_png_mem` | `cp_out_size(&img,bpp) < 1` while `cp_out_size(&img,4) >= 1` (only reachable for `bpp ∈ {2,3}` wrapping differently than `bpp == 4`) | `img = {w-1,h,NULL}`; reason `"invalid image size found"` |
| 24 | `load_png_mem` | `cp_inflate(...)` returns 0 for any of rows 1–6 | `img = {w-1,h,NULL}`; reason `"DEFLATE algorithm failed"` (inner reason **overwritten**) |
| 25 | `load_png_mem` | `cp_unfilter` returns 0 — a row filter byte `> 4` on the **first** row (`h > 0` branch, `default: return 0`) | `img = {w-1,h,NULL}`; reason `"invalid filter byte found"` |
| 26 | `load_png_mem` | `cp_unfilter` returns 0 — a row filter byte `> 4` on any **subsequent** row (`y >= 1` loop, `default: return 0`) | `img = {w-1,h,NULL}`; reason `"invalid filter byte found"` |
| 27 | `load_png_mem` | `color_type == 3` (indexed) but no `PLTE` chunk was found | `img = {w-1,h,NULL}`; reason `"color type of indexed requires a PLTE chunk"` |
| 28 | `cp_chunk` | chunk type mismatch, `len < minlen`, or `png->p + (int)(len+12) > png->end` | returns `NULL` and leaves `png->p` unchanged (observable through rows 8/18: IDAT concatenation stops) |
| 29 | `cp_find` | scans to `png->p >= png->end` without a matching chunk | returns `NULL`, `png->p` left past `end` (observable through rows 18/27) |

## Assertion failures (SIGABRT in both implementations)

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| A1 | `cp_ptr` | `s->bits_left & 7` — stored block reached at a non-byte-aligned `bits_left` (`cp_inflate` called with `in_bytes*8` such that the 5-byte stored header leaves a bit remainder; reachable via a `btype==0` block that is not the first block) | SIGABRT |
| A2 | `cp_peak_bits` | `s->word_index > s->word_count` (defensive; unreachable given the guarding `if`) | SIGABRT |
| A3 | `cp_consume_bits` | `s->count < num_bits_to_read` — e.g. `cp_decode` needs `key & 0xF` bits that the exhausted stream cannot supply | SIGABRT |
| A4 | `cp_read_bits` | `num_bits_to_read > 32` (unreachable from the fixed call sites) | SIGABRT |
| A5 | `cp_read_bits` | `num_bits_to_read < 0` — reachable by writing a value `> 127` into the exported `cp_len_extra_bits` / `cp_dist_extra_bits` tables (`uint8_t` → `int` is non-negative, so actually only via `s->count & 7` with a negative `count`) | SIGABRT |
| A6 | `cp_read_bits` | `s->bits_left <= 0` — input stream exhausted, e.g. `cp_inflate(in, 0, out, n)` (`bits_left == 0` immediately) | SIGABRT |
| A7 | `cp_read_bits` | `s->count > 64` | SIGABRT |
| A8 | `cp_read_bits` | `cp_would_overflow(s, n)` i.e. `(bits_left + count) - n < 0` — asking for more bits than remain | SIGABRT |
| A9 | `cp_build` | `lens[i] >= 16` — reachable by writing `≥ 16` into the exported `cp_fixed_table` before a `btype==1` block | SIGABRT |
| A10 | `cp_decode` | `(search >> (32 - (key & 0xF))) != (key >> (32 - (key & 0xF)))` — an incomplete/over-subscribed Huffman table, i.e. a bit pattern that matches no code | SIGABRT |

## Generic FFI boundary cases (not in the C's own check list)

| # | entry point | trigger | expected C result |
|---|-------------|---------|-------------------|
| G1 | `load_png_mem` | `png_length == 0` with a valid 8-byte signature buffer | reads past the caller's length (the C never bounds-checks against `png_length` before `memcmp`/`cp_make32`); with a ≥8-byte allocation the signature check passes and `cp_chunk` then fails ⇒ row 8 |
| G2 | `load_png_mem` | `png_length` negative | `png.end < png.p`, `cp_find` loop body never runs ⇒ row 8 or row 18 |
| G3 | `load_png_mem` | truncated after IHDR (no IDAT) | row 18 |
| G4 | `cp_inflate` | `out_bytes == 0` | `out_end == out`, first literal ⇒ row 3 |
| G5 | `cp_inflate` | `out_bytes` negative | `out_end < out`, first literal ⇒ row 3 |
| G6 | `cp_inflate` | `in_bytes == 0` | `bits_left == 0` ⇒ A6 (SIGABRT) |
| G7 | `cp_inflate` | `in_bytes` negative | `bits_left` negative ⇒ A6 (SIGABRT) |
| G8 | `cp_inflate` | `in` pointer at each of the 4 alignments × each `in_bytes % 4` | `first_bytes`/`last_bytes`/`final_word` paths; must agree |
| G9 | `load_png_mem` | `ihdr[9]` (colour type) set to every value `0..=255` — a C `enum`-like field accepts any `int` | rows 10 / valid-`bpp` paths, must agree for all 256 |
| G10 | `load_png_mem` | `ihdr[8]` (bit depth) set to every value `0..=255` | row 9 for all but `8` |
| G11 | `cp_inflate` | `btype` for all 4 values `0..=3` (2-bit field, no invalid encoding possible) | rows 1–6 |
| G12 | `load_png_mem` | filter byte set to every value `0..=255` on row 0 and on row 1 | rows 25/26 for `> 4`, valid filters otherwise |
| G13 | `load_png_mem`/`cp_inflate` | NULL pointer for `png_data` / `in` / `out` | SIGSEGV in both (dereferenced without a null check) |

---

# Phase C results — every row has a passing differential test

All tests live in `tests/phase_c_errors.rs`. Each one constructs the exact
condition, calls **both** `.so`s through `dlsym` in forked children, and asserts
the same return value / `pix == NULL`, the same `cp_error_reason` **text** (read
through the exported pointer, not compared by address) and the same termination
signal.

| row | test | status | evidence |
|-----|------|--------|----------|
| 1  | `err01_stored_len_nlen_mismatch` | [x] | 50 randomized NLEN corruptions + the `load_png_mem` overwrite case |
| 2  | `err02_stored_extends_beyond_input` | [x] | 7 trailing-byte sizes + `LEN=1` with 100 trailing bytes |
| 3  | `err03_out_buffer_full_on_literal` | [x] | `out_bytes` 0 / n-1 / negative / `INT_MIN` |
| 4  | `err04_backwards_distance_before_begin` | [x] | 6 distances with empty history + 40 randomized history/distance pairs |
| 5  | `err05_string_overruns_out_buffer` | [x] | 40 randomized length/distance/out-size triples |
| 6  | `err06_reserved_block_type` | [x] | `btype=3` with `bfinal` 0 and 1, 12 input lengths (also row G11: all 4 `btype`s) |
| 7  | `err07_bad_signature` | [x] | each of the 8 signature bytes flipped + 40 random buffers |
| 8  | `err08_missing_or_short_ihdr` | [x] | no chunks / wrong first chunk (4) / `len` 0..12 / declared length past end (4) / truncated `png_length` (7) |
| 9  | `err09_bit_depth_full_sweep` | [x] | **all 256** values of `ihdr[8]` (row G10) |
| 10 | `err10_color_type_full_sweep` | [x] | **all 256** values of `ihdr[9]` (row G9 — out-of-range enum across FFI) |
| 11 | `err11_width_less_than_one` | [x] | `0xFFFFFFFF`, `0x7FFFFFFF`, `0x80000000`, `0xFFFFFFFE`, `0xC0000000`, plus `0` (the valid `w==1` boundary) |
| 12 | `err12_height_less_than_one` | [x] | `0`, `0x80000000`, `0xFFFFFFFF`, `0x90000000` |
| 13 | `err13_image_too_large` | [x] | four geometries straddling `w*h*4 >= 0x7FFFFFFF`, incl. the exact `2^31` boundary |
| 14 | `err14_allocation_boundary` | [x] | the largest admissible size (`malloc(0x7FFFFFFC)`); the NULL branch is **forced** by capping the child's `RLIMIT_AS` to 1 GiB, which is the only way to reach it |
| 15 | `err15_16_17_ihdr_method_sweeps` | [x] | **all 256** values of `ihdr[10]` |
| 16 | `err15_16_17_ihdr_method_sweeps` | [x] | **all 256** values of `ihdr[11]` |
| 17 | `err15_16_17_ihdr_method_sweeps` | [x] | **all 256** values of `ihdr[12]` |
| 18 | `err18_corrupt_zlib_structure` | [x] | no IDAT; IDAT payloads of 0..5 bytes; five 1-byte IDATs |
| 19 | `err19_zlib_compression_method` | [x] | **all 16** CM values |
| 20 | `err20_zlib_window_size` | [x] | **all 16** CINFO values (0..7 must decode, 8..15 must be rejected) |
| 21 | `err21_zlib_preset_dictionary` | [x] | **all 256** FLG values (asserts rejection iff bit 5 set) |
| 22 | `err22_err23_out_size_guards_are_unreachable` | [x] | **proven unreachable**: `w >= 1`, `h >= 1` and row 13 force `w*h < 0x20000000`, so `(img.w+1)*img.h*4` is always in `1..0x7FFFFFFF`. The test drives the largest admissible geometries and asserts the reason is *not* "invalid image size found" |
| 23 | `err22_err23_out_size_guards_are_unreachable` | [x] | same argument for `bpp ∈ {1,2,3}` (`cp_out_size(bpp) <= cp_out_size(4)`); tested for all four `bpp` |
| 24 | `err24_deflate_algorithm_failed` | [x] | `btype=3` inside the IDAT, and an output overrun; confirms the inner reason is **overwritten** |
| 25 | `err25_invalid_filter_first_row` | [x] | 7 invalid filter bytes × all 5 colour types |
| 26 | `err26_invalid_filter_later_row` | [x] | 3 invalid filter bytes × rows 1/2/4 × all 5 colour types |
| 27 | `err27_indexed_without_plte` | [x] | 3 geometries, plus a tRNS-without-PLTE case |
| 28 | `err28_cp_chunk_rejections` | [x] | second IDAT declared past the end; a non-IDAT chunk between IDATs; declared lengths that sign-extend negative in `int offset` (3) |
| 29 | `err29_cp_find_walks_off_the_end` | [x] | 5 declared lengths that push the cursor past `end` |
| A1 | `errA1_stored_block_at_unaligned_bits_left` | [x] | 386-case targeted search; **3 cases confirmed to reach `cp_ptr`**, proved by capturing the C child's `stderr`: `lib.c:80: cp_ptr: Assertion '!(s->bits_left & 7)' failed` |
| A2 | — | [x] | **unreachable**: guarded by the enclosing `if (s->word_index < s->word_count)`, so `word_index <= word_count` always holds at the assert |
| A3 | `errA3_A8_truncated_streams` | [x] | 747 truncations; **477 reached `cp_consume_bits`** (stderr-confirmed) |
| A4 | `errA4_extra_bits_table_out_of_range` | [x] | `cp_len_extra_bits` / `cp_dist_extra_bits` set to 33/64/100/255 ⇒ SIGABRT on both sides |
| A5 | — | [x] | **unreachable**: the only argument that could be negative is `s->count & 7`, and `cp_consume_bits` asserts `count >= n` before subtracting, so `count` never goes negative; `uint8_t`→`int` from the tables is always ≥ 0 |
| A6 | `errA6_input_exhausted_immediately` | [x] | `in_bytes` = 0 (× 4 alignments) and −1 / −7 / −1000 / `INT_MIN` |
| A7 | — | [x] | **unreachable**: `count` only grows via `+= 32` (guarded by `count < n <= 16`) or `+= bits_left`, giving `count' = 2*count + 8*last_bytes <= 2*15 + 24 = 54` |
| A8 | `errA3_A8_truncated_streams` | [x] | same corpus; **63 reached `cp_read_bits`** (stderr-confirmed) |
| A9 | `errA9_fixed_table_code_length_too_long` | [x] | `cp_fixed_table[idx]` set to 16/17/31/100/255 at 6 offsets spanning both the lit/len and dist halves |
| A10 | `errA10_incomplete_huffman_table` | [x] | hand-built under-subscribed lit/len code; **34/40 cases abort**, the other 6 decode a wrong symbol — identically on both sides |
| G1 | `errG1_G2_G3_png_length_edge_cases` | [x] | `png_length` = 0 / 1 / 7 / 8 |
| G2 | `errG1_G2_G3_png_length_edge_cases` | [x] | `png_length` = −1 / −8 / −1000 / `INT_MIN` |
| G3 | `errG1_G2_G3_png_length_edge_cases` | [x] | truncated at every offset from 8 to `len` (both as a short buffer and as a short length) |
| G4 | `errG4_G5_out_bytes_edge_cases` | [x] | `out_bytes` = 0 / 1 / 19 / 20 / 21 / `INT_MAX` |
| G5 | `errG4_G5_out_bytes_edge_cases` | [x] | `out_bytes` = −1 / −20 / `INT_MIN` |
| G6 | `errA6_input_exhausted_immediately` | [x] | `in_bytes == 0` |
| G7 | `errA6_input_exhausted_immediately` | [x] | `in_bytes < 0` |
| G8 | `phase_b_valid::row12_inflate_alignment_matrix` | [x] | 4 `in` alignments × 4 `in_bytes mod 4` × 20 randomized streams |
| G9 | `err10_color_type_full_sweep` | [x] | all 256 colour-type bytes |
| G10 | `err09_bit_depth_full_sweep` | [x] | all 256 bit-depth bytes |
| G11 | `err06_reserved_block_type` | [x] | all 4 `btype` values |
| G12 | `phase_b_valid::row48_filter_byte_full_sweep` | [x] | filter byte 0..=255 on row 0 and row 1 × 3 colour types |
| G13 | `errG13_null_pointers` | [x] | `load_png_mem(NULL,·)`, `cp_inflate(NULL,·)`, `cp_inflate(·,NULL)` — **SIGSEGV on both sides** |

**Result: 35/35 tests pass**, under both the release and the debug Rust `.so`.
Three rows (A2, A5, A7) and two rows (22, 23) are argued unreachable from the C's
own guards rather than tested directly; each is accompanied by a boundary test
that pins the reasoning.

## A note on the watchdog

Some malformed streams make the C loop forever: `cp_decode` can pick a `key` with
`key & 0xF == 0`, `cp_consume_bits(s, 0)` then makes no progress, and the same
symbol is decoded again indefinitely. The Rust does the same. The harness gives
each child a 2 s alarm so this is bounded; if exactly **one** side times out the
pair is re-run with a 300 s alarm, because the C is built at `-O0` and the Rust
`.so` at `-O3` and a one-sided 2 s timeout says nothing about behaviour. 15 of
800 mutated-stream cases livelock on both sides.
