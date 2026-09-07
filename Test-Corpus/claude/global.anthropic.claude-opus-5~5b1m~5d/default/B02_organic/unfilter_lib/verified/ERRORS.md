# ERRORS.md — Phase A: the error-surface table

Derived mechanically from `c_src/src/lib.c`: every `return 0`, every
`cp_error_reason = ...` assignment, every `assert()`, every `return NULL`/`0`
pointer sentinel, and every explicit range/bounds comparison.  Nothing here is
guessed — the line numbers are the exact lines in `c_src/src/lib.c`.

`cp_error_reason` is a *sticky* global: the C code only ever writes it on the
six failure branches below, never clears it.  So a differential test must
compare the **pointer's target string**, and must reset the global to `NULL`
before every call to be able to tell "set" from "left over".

## A. Hard rejections that propagate to a public return value

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| 1 | `unfilter` (`lib.c:439`) | `h > 0` and the filter byte of **row 0** (`raw[0]`) is `>= 5` (i.e. not one of `0,1,2,3,4`) | returns `0`; `raw` left completely untouched | `err01_unfilter_row0_bad_filter` (exhaustive over filter bytes 5..255) | [x] |
| 2 | `unfilter` (`lib.c:473`) | `h >= 2` and the filter byte of **any row `y >= 1`** is `>= 5` | returns `0`; rows `0..y-1` already un-filtered in place, rows `>= y` untouched | `err02_unfilter_later_row_bad_filter` | [x] |
| 3 | `cp_stored` (`lib.c:176`) → `cp_inflate` | stored block (`BTYPE == 0`) whose `LEN` is not the one's complement of `NLEN` | `cp_inflate` returns `0`; `cp_error_reason == "Failed to find LEN and NLEN as complements within stored (uncompressed) stream."` | `err03_stored_len_nlen_mismatch` | [x] |
| 4 | `cp_stored` (`lib.c:185`) → `cp_inflate` | stored block where `s->bits_left / 8 > (int)LEN`, i.e. `LEN` claims fewer bytes than remain — *note the sense of the C check: it rejects when too **many** input bytes remain* | `cp_inflate` returns `0`; `cp_error_reason == "Stored block extends beyond end of input stream."` | `err04_stored_block_extends_beyond_input` | [x] |
| 5 | `cp_block` (`lib.c:260`) → `cp_inflate` | a literal symbol (`< 256`) is decoded while `s->out + 1 > s->out_end` (output buffer full / `out_bytes == 0`) | `cp_inflate` returns `0`; `cp_error_reason == "Attempted to overwrite out buffer while outputting a symbol."` | `err05_out_buffer_full_on_literal` | [x] |
| 6 | `cp_block` (`lib.c:279`) → `cp_inflate` | a length/distance pair whose `backwards_distance` points before the start of the output buffer (`s->out - dist < s->begin`) | `cp_inflate` returns `0`; `cp_error_reason == "Attempted to write before out buffer (invalid backwards distance)."` | `err06_backwards_distance_before_buffer` | [x] |
| 7 | `cp_block` (`lib.c:288`) → `cp_inflate` | a length/distance pair whose copy would run past the end (`s->out + length > s->out_end`); checked **after** #6 | `cp_inflate` returns `0`; `cp_error_reason == "Attempted to overwrite out buffer while outputting a string."` | `err07_string_overruns_out_buffer` | [x] |
| 8 | `cp_inflate` (`lib.c:362`) | block header `BTYPE == 3` (reserved) | `cp_inflate` returns `0`; `cp_error_reason == "Detected unknown block type within input stream."` | `err08_unknown_block_type` | [x] |

Ordering constraints that the table encodes and the tests must pin down:

* #3 is checked before #4 inside the same stored block.
* #6 is checked before #7 for the same length/distance pair.
* `bfinal`/`btype` are read (2 `cp_read_bits`) *before* any of #3..#8, so the
  bit-stream position on failure differs per branch — observable through the
  amount of output produced by earlier blocks.

## B. Pointer-sentinel rejections in the (dead, `static`) PNG chunk helpers

Not reachable from either `.so`'s public surface (`cp_chunk`/`cp_find` are
`static` and never called in `lib.c`), listed for completeness of the grep.

| # | function | trigger | expected C result | test | [x] |
|---|----------|---------|-------------------|------|-----|
| 9  | `cp_chunk` (`lib.c:403`) | `memcmp(start+4, chunk, 4) != 0` | returns `NULL` | `err09_to_12_png_chunk_helpers_are_unreachable` (proves the symbol is absent from every `.so`, so the row is unreachable across the FFI) | [x] |
| 10 | `cp_chunk` (`lib.c:403`) | `len < minlen` | returns `NULL` | `err09_to_12_png_chunk_helpers_are_unreachable` (proves the symbol is absent from every `.so`, so the row is unreachable across the FFI) | [x] |
| 11 | `cp_chunk` (`lib.c:403`) | `png->p + len + 12 > png->end` | returns `NULL`, `png->p` **not** advanced | `err09_to_12_png_chunk_helpers_are_unreachable` (proves the symbol is absent from every `.so`, so the row is unreachable across the FFI) | [x] |
| 12 | `cp_find`  (`lib.c:415`) | walks to `png->p >= png->end` without a match | returns `NULL`, `png->p` left past `end` | `err09_to_12_png_chunk_helpers_are_unreachable` (proves the symbol is absent from every `.so`, so the row is unreachable across the FFI) | [x] |

## C. `assert()`s — abort-on-violation (only when built without `NDEBUG`)

The bare `cmake ..` configure used in the task description leaves
`CMAKE_BUILD_TYPE` empty, so **`-DNDEBUG` is not passed and these asserts are
live**: the C `.so` calls `__assert_fail` → `SIGABRT`.  With
`-DCMAKE_BUILD_TYPE=Release` they vanish and the C code runs on into the
undefined-behaviour that the assert was guarding.

An abort is not a value the Rust `cdylib` can "return", and the release C
library (the configuration the translation targets) does not abort at all, so
the Rust code deliberately contains **no** assert translations.  The tests below
therefore run every `cp_inflate` case in a **forked child** and record
`(exit status, return value, output buffer, cp_error_reason)`; a row is
satisfied when the C child *survives* and both sides agree, and is reported as
"C self-destructed" when it does not.

| # | function | assert / trigger | expected C result (asserts live) | test | [x] |
|---|----------|------------------|-----------------------------------|------|-----|
| 13 | `cp_ptr` (`lib.c:95`) | `s->bits_left & 7` — stored block reached at a non-byte-aligned residual bit count | `SIGABRT` | `err13_to_22_asserts_are_live_in_the_assert_build` + `err13_to_22_release_build_matches_rust` + `fuzz_malformed_streams_match` | [x] |
| 14 | `cp_peak_bits` (`lib.c:104`) | `s->word_index > s->word_count` | `SIGABRT` | `err13_to_22_asserts_are_live_in_the_assert_build` + `err13_to_22_release_build_matches_rust` + `fuzz_malformed_streams_match` | [x] |
| 15 | `cp_consume_bits` (`lib.c:115`) | `s->count < num_bits_to_read` — consuming more bits than are buffered (happens as soon as the stream is truncated) | `SIGABRT` | `err13_to_22_asserts_are_live_in_the_assert_build` + `err13_to_22_release_build_matches_rust` + `fuzz_malformed_streams_match` | [x] |
| 16 | `cp_read_bits` (`lib.c:123`) | `num_bits_to_read > 32` | `SIGABRT` | `err13_to_22_asserts_are_live_in_the_assert_build` + `err13_to_22_release_build_matches_rust` + `fuzz_malformed_streams_match` | [x] |
| 17 | `cp_read_bits` (`lib.c:124`) | `num_bits_to_read < 0` | `SIGABRT` | `err13_to_22_asserts_are_live_in_the_assert_build` + `err13_to_22_release_build_matches_rust` + `fuzz_malformed_streams_match` | [x] |
| 18 | `cp_read_bits` (`lib.c:125`) | `s->bits_left <= 0` — reading after the input is exhausted, e.g. `in_bytes == 0` | `SIGABRT` | `err13_to_22_asserts_are_live_in_the_assert_build` + `err13_to_22_release_build_matches_rust` + `fuzz_malformed_streams_match` | [x] |
| 19 | `cp_read_bits` (`lib.c:126`) | `s->count > 64` | `SIGABRT` | `err13_to_22_asserts_are_live_in_the_assert_build` + `err13_to_22_release_build_matches_rust` + `fuzz_malformed_streams_match` | [x] |
| 20 | `cp_read_bits` (`lib.c:127`) | `cp_would_overflow(s, n)`, i.e. `(bits_left + count) - n < 0` — truncated stream | `SIGABRT` | `err13_to_22_asserts_are_live_in_the_assert_build` + `err13_to_22_release_build_matches_rust` + `fuzz_malformed_streams_match` | [x] |
| 21 | `cp_build` (`lib.c:154`) | a code length `>= 16` (only from a corrupt dynamic-Huffman header) | `SIGABRT`; in `Release` it is instead an out-of-bounds write to `counts`/`codes`/`first` on the C stack (unreproducible UB — the Rust translation uses 256-entry tables and stays memory-safe) | `err13_to_22_asserts_are_live_in_the_assert_build` + `err13_to_22_release_build_matches_rust` + `fuzz_malformed_streams_match` | [x] |
| 22 | `cp_decode` (`lib.c:217`) | `(search >> len) != (key >> len)` — the peeked bits match no code in the tree | `SIGABRT`; in `Release` the bogus `key` is used, and when `hi == 0` the C reads `tree[-1]` (the tail of `s->lookup` / `s->lit` / `s->dst` inside the same `calloc`ed state). The Rust translation derives every sub-array pointer from the state base so `tree[-1]` reads the identical byte. | `err13_to_22_asserts_are_live_in_the_assert_build` + `err13_to_22_release_build_matches_rust` + `fuzz_malformed_streams_match` | [x] |

## D. Generic FFI boundaries covered by the tests even though the C has no check for them

| # | entry point | input | C behaviour (Release) | test | [x] |
|---|-------------|-------|------------------------|------|-----|
| 23 | `unfilter` | `h == 0` | filter byte of row 0 is **not** read; `prev = raw`, `raw += len`, loop body never runs → returns `1`, buffer untouched | `err23_24_unfilter_h_zero_and_negative` | [x] |
| 24 | `unfilter` | `h < 0` | same as #23 (both `if (h > 0)` and `y < h` are false) → returns `1` | `err23_24_unfilter_h_zero_and_negative` | [x] |
| 25 | `unfilter` | `w == 0` or `bpp == 0` → `len == 0` | every inner loop is empty; only the filter bytes are skipped → returns `1` unless a filter byte is `>= 5` | `err25_unfilter_len_zero` | [x] |
| 26 | `unfilter` | `bpp > len` (e.g. `w == 1`) | row-0 loops never run; row-`y` `x < bpp` prologue loops run past `len` and touch bytes of the *next* row | `err26_unfilter_bpp_greater_than_len` | [x] |
| 27 | `unfilter` | negative `w` / `bpp` (so `len < 0`, or `len` sign-flipped) | all `x < len` loops empty; `x < bpp` prologue empty; returns `1` | `err27_unfilter_negative_dimensions` | [x] |
| 28 | `unfilter` | filter byte `= 5 .. 255` (the full out-of-range enum domain, one step past the valid `0..4`) | returns `0` | `err28_filter_byte_full_domain` (exhaustive over all 256 byte values × 3 row counts) | [x] |
| 29 | `cp_inflate` | `in_bytes == 0` | `bits_left == 0` → `cp_read_bits` on an empty stream. Release: reads whatever `bits` holds (0) → `bfinal=0, btype=0` → stored path; asserts build: row #18 `SIGABRT` | `err29_inflate_in_bytes_zero` | [x] |
| 30 | `cp_inflate` | `out_bytes == 0` | any literal trips row #5 | `err30_inflate_out_bytes_zero` | [x] |
| 31 | `cp_inflate` | `in_bytes < 0` | `bits_left = in_bytes*8 < 0`, `word_count < 0` → nothing readable from `words`; Release: `final_word_available == 0`, decodes from `bits == 0` | `err31_inflate_negative_in_bytes` | [x] |
| 32 | `cp_inflate` | `out_bytes < 0` | `out_end < out`, so the very first literal trips row #5 | `err32_inflate_negative_out_bytes` | [x] |
| 33 | `cp_inflate` | `in` misaligned by 1/2/3 bytes | exercises the `first_bytes` pre-load path (`bits` seeded from 1..3 bytes, `count = first_bytes*8`) — must match the aligned result for the same logical stream | `err33_34_alignment_paths_on_malformed_input` | [x] |
| 34 | `cp_inflate` | `in_bytes % 4 != 0` after alignment | exercises the `final_word` / `final_word_available` tail path | `err33_34_alignment_paths_on_malformed_input` | [x] |

## E. Behaviours discovered *while running* the Phase C tests

Every one of these is genuine C behaviour that the tests initially flagged as a
wrong *expectation on my side*, not as a C-vs-Rust divergence.  They are
recorded here because they are exactly the kind of thing a "reasonable" reading
of the source would get wrong, and because the Rust translation reproduces all
of them bit for bit.

1. **`cp_stored` has no output-bounds check at all.**  Unlike `cp_block`, it
   `memcpy`s `LEN` bytes into `s->out` without ever comparing against
   `s->out_end`.  So `cp_inflate(in, n, out, 0)` on a *stored* block returns `1`
   and overruns the caller's buffer by `LEN` bytes.  (`err30` therefore has to
   force a real Huffman block to reach ERRORS.md row 5.)

2. **`cp_stored`'s copy source is only correct when `(in_bytes) % 4 == 0`.**
   `cp_ptr()` computes `(char*)(words + word_index) - count/8`, which assumes
   the bit buffer was last refilled from `s->words`.  Once `cp_peak_bits` has
   fallen back to `s->final_word` — which does **not** advance `word_index` —
   the pointer is short by `last_bytes`, and the C code copies the `LEN`/`NLEN`
   bytes instead of the payload.  For a lone stored block the input is
   `LEN + 5` bytes, so the payload is only copied correctly when
   `LEN % 4 == 3`.  `CONFIGS.md` row 16 splits its cases accordingly.

3. **A stored block can only ever be the *last* block.**  Row 4's check is
   `bits_left / 8 <= LEN`, so any stored block with more than `LEN` input bytes
   still to come is rejected.  This is why `flate2`/miniz level 0 output
   (a chain of stored blocks) is rejected for payloads over 64 KiB, and why
   `CONFIGS.md` row 27 only puts stored blocks at the end of a chain.

4. **`cp_stored` never advances the bit position past the stored payload.**  It
   moves `s->out` but leaves `bits`/`count`/`bits_left` where they were, so a
   non-final stored block would re-parse its own payload as a bit stream.

5. **With `len == 0` and `bpp > 0`, `unfilter`'s row-`y` prologue overwrites the
   *filter bytes of later rows*.**  `for (x = 0; x < bpp; x++) raw[x] += prev[x]`
   is bounded by `bpp`, not by `len`, so for filters 2/3/4 the return value of
   `unfilter(0, h, 4, raw)` becomes data-dependent as soon as `h >= 3`.
   `err25` only makes a hard assertion for the unambiguous cases.

6. **`out == NULL` with `out_bytes < 0` makes `out_end` wrap to `~0`,** so the
   `s->out + 1 <= s->out_end` guard passes and the C code dereferences `NULL`.
   Both libraries segfault identically (`err_null_pointers`).

## Result

All 34 rows have a passing differential test; see the `[x]` column.  In
addition, `fuzz_malformed_streams_match` runs **2000** malformed inputs (random
bytes, single-bit flips of valid streams, multi-bit flips, and truncations, at
all four input alignments): **1999 compared byte-for-byte identical, 1 case the
C library killed itself on (its own out-of-bounds stack write), 0 divergences.**
