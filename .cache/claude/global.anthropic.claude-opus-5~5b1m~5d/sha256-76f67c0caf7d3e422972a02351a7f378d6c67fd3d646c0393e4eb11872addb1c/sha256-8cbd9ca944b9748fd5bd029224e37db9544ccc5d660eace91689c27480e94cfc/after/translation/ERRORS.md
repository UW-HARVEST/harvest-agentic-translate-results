# ERRORS.md — error-surface table (Phase C)

Mechanically derived from `c_src/src/lib.c`: every `cp_error_reason = ...` /
`goto cp_err`, every `return 0` / `return NULL` rejection, every `assert()`,
and every range/bounds check. Line numbers refer to `c_src/src/lib.c`.

`R0` below means "`cp_inflate` returns 0". `SIGABRT` means the process dies via
glibc `assert` -> `abort()` (the C library is compiled without `NDEBUG`, see
`CMakeLists.txt`, so all asserts are live).

## A. `cp_error_reason`-setting rejections (reachable from `cp_inflate`)

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `cp_stored` (L170-176) | stored block (`BTYPE=00`) whose `LEN` is not the one's complement of `NLEN` | `R0`, `cp_error_reason = "Failed to find LEN and NLEN as complements within stored (uncompressed) stream."` |
| 2 | `cp_stored` (L179-185) | stored block where `s->bits_left / 8 > LEN`, i.e. more than `LEN` bytes still remain in the input after the 4-byte LEN/NLEN header | `R0`, `cp_error_reason = "Stored block extends beyond end of input stream."` |
| 3 | `cp_block` (L254-261) | literal symbol decoded (`sym < 256`) while `s->out + 1 > s->out_end` — output buffer full / `out_bytes` too small | `R0`, `cp_error_reason = "Attempted to overwrite out buffer while outputting a symbol."` |
| 4 | `cp_block` (L273-279) | back-reference whose distance points before the start of the output buffer (`out - backwards_distance < begin`) | `R0`, `cp_error_reason = "Attempted to write before out buffer (invalid backwards distance)."` |
| 5 | `cp_block` (L282-289) | back-reference copy that would run past the end of the output buffer (`out + length > out_end`) | `R0`, `cp_error_reason = "Attempted to overwrite out buffer while outputting a string."` |
| 6 | `cp_inflate` (L354-362) | `BTYPE == 3` (reserved block type) | `R0`, `cp_error_reason = "Detected unknown block type within input stream."` |

Note on #6: `BTYPE` is read with `cp_read_bits(s, 2)`, so the `switch` in
`cp_inflate` can only see 0/1/2/3 — there is no `default:` arm and no
out-of-range `int` can reach it from outside. The equivalent "out-of-range enum
value across the FFI boundary" for this library is the `bpp` argument of
`convert_pix` (an unconstrained `int` used as a `switch` selector); that is
covered by rows 27-29.

## B. Rejections in `static` helpers not reachable from an exported symbol

No exported entry point reaches these, so they have no *dynamic* symbol — but
`c_src` is built at `-O0`, so the C `.so` retains them as LOCAL symbols and
`tests/private.rs` calls them by resolving `base + nm_offset(name)`, with the
Rust side reached through the `private_probe` test-only cdylib. Every row below
therefore HAS a passing differential test.

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 7 | `cp_unfilter` (L433) | first scanline filter byte `> 4` | `return 0` |
| 8 | `cp_unfilter` (L467) | any later scanline filter byte `> 4` | `return 0` |
| 9 | `cp_chunk` (L397) | 4-byte chunk tag at `p+4` != requested tag | `return NULL` |
| 10 | `cp_chunk` (L397) | chunk `len < minlen` | `return NULL` |
| 11 | `cp_chunk` (L397) | `p + len + 12 > end` (chunk runs off the buffer) | `return NULL` |
| 12 | `cp_find` (L409) | no chunk with the requested tag / `len >= minlen` / in-bounds before `p >= end` | `return NULL` |

## C. `assert()` failures — abort, not a return value

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 13 | `cp_read_bits` (L119) `assert(s->bits_left > 0)` | `cp_inflate` called with `in_bytes <= 0` (empty input): the very first `cp_read_bits(s, 1)` sees `bits_left == 0` | `SIGABRT`, stderr `.../lib.c:119: cp_read_bits: Assertion \`s->bits_left > 0' failed.` |
| 14 | `cp_read_bits` (L121) `assert(!cp_would_overflow(s, n))` | truncated stream: fewer bits remain (`bits_left + count < n`) than the header/extra-bits read needs — e.g. a 1-byte input whose first bit says `BFINAL=0` and then a second block header is requested | `SIGABRT`, `.../lib.c:121: cp_read_bits: Assertion \`!cp_would_overflow(s, num_bits_to_read)' failed.` |
| 15 | `cp_consume_bits` (L109) `assert(s->count >= num_bits_to_read)` | `cp_peak_bits` could not top the bit accumulator up to `num_bits_to_read` (input exhausted mid-symbol / a 16-bit `LEN`/`NLEN` read in `cp_stored` with fewer than 16 buffered bits) | `SIGABRT`, `.../lib.c:109: cp_consume_bits: Assertion \`s->count >= num_bits_to_read' failed.` |
| 16 | `cp_decode` (L211) `assert((search >> len) == (key >> len))` | the peeked bits do not match any code in the Huffman tree (corrupt/garbage compressed data) | `SIGABRT`, `.../lib.c:211: cp_decode: Assertion \`(search >> len) == (key >> len)' failed.` |
| 17 | `cp_build` (L148) `assert(len < 16)` | a code-length array entry `>= 16` — reachable from `cp_dynamic` when the code-length Huffman tree decodes a literal length symbol `>= 16`, or by a caller tampering with the exported `cp_fixed_table` | `SIGABRT`, `.../lib.c:148: cp_build: Assertion \`len < 16' failed.` |
| 18 | `cp_ptr` (L89) `assert(!(s->bits_left & 7))` | `cp_stored` reaching `cp_ptr` with `bits_left` not a multiple of 8. Unreachable in practice: `cp_stored` first does `cp_read_bits(s, s->count & 7)` and then two 16-bit reads, which leaves `bits_left` byte-aligned whenever `in_bytes*8` was (always). Kept in the table and asserted to be non-triggering. |
| 19 | `cp_peak_bits` (L98) `assert(s->word_index <= s->word_count)` | `word_index` overrunning `word_count`; guarded by the enclosing `if (s->word_index < s->word_count)`, so unreachable. Kept for completeness. |
| 20 | `cp_read_bits` (L117/L118) `assert(n <= 32)`, `assert(n >= 0)` | a negative or `> 32` bit count. All call sites pass literals `1,2,3,4,5,7,16`, `s->count & 7` (0..7), or a table entry from `cp_len_extra_bits`/`cp_dist_extra_bits` (0..13), so unreachable from a well-formed call — but reachable if the caller overwrites the exported `cp_len_extra_bits` / `cp_dist_extra_bits` tables with a value `> 32`. Covered by tampering the exported table. |
| 21 | `cp_read_bits` (L120) `assert(s->count <= 64)` | `count` exceeding 64; `count` only grows by 32 when `count < num_bits_to_read <= 32`, so unreachable. Kept for completeness. |

## D. Generic FFI boundary conditions (not distinct C branches, tested anyway)

| # | entry point | trigger | expected C result |
|---|-------------|---------|-------------------|
| 22 | `cp_inflate` | `in_bytes == 0` (with a non-null, and with a null, `in`) | row 13 (`SIGABRT`) |
| 23 | `cp_inflate` | `in_bytes < 0` (negative length) | row 13 (`SIGABRT`) — `bits_left = in_bytes*8 < 0` fails `assert(s->bits_left > 0)` |
| 24 | `cp_inflate` | `out_bytes == 0` with a stream that emits at least one byte | row 3 |
| 25 | `cp_inflate` | `out_bytes < 0` (`out_end < out`) | row 3 |
| 26 | `cp_inflate` | valid stream, `out` buffer exactly 1 byte too small | row 3 or row 5 |
| 27 | `convert_pix` | `bpp` outside `{1,2,3,4}`: `0`, `5`, `-1`, `INT_MIN`, `INT_MAX`, `256` (the "out-of-range enum" case) | no store to `dst`; `src` still advances by `bpp` per pixel; returns void |
| 28 | `convert_pix` | `w <= 0` or `h <= 0` (`0`, `-1`, `INT_MIN`) | loops do not execute; no store to `dst`; returns void |
| 29 | `convert_pix` | `w == 0`, `h > 0` | `src` advanced by 1 per row only, `dst` untouched |
| 30 | `cp_error_reason` | read before any call (fresh library) | `NULL` in both |
| 31 | `cp_error_reason` | not cleared on success — keeps the last error string after a subsequent *successful* `cp_inflate` | identical stale pointer contents in both |

## Checklist

- [x] 1 — `tests/errors.rs::err01_stored_len_nlen_mismatch`
- [x] 2 — `tests/errors.rs::err02_stored_extends_beyond_input`
- [x] 3 — `tests/errors.rs::err03_out_overflow_symbol`
- [x] 4 — `tests/errors.rs::err04_invalid_backwards_distance`
- [x] 5 — `tests/errors.rs::err05_out_overflow_string`
- [x] 6 — `tests/errors.rs::err06_unknown_block_type`
- [x] 7-8  — `tests/private.rs::priv_cp_unfilter_all_filters` (every per-row filter byte 0..6 x bpp 1..4 x w/h grid) and `priv_cp_unfilter_degenerate_shapes`
- [x] 9-11 — `tests/private.rs::priv_cp_chunk` (matching/mismatching 4-byte tags, `minlen` above and below `len`, `end` cut short, and `len` values around `0x7FFFFFF4` / `0xFFFFFFF4` where `len + 12` crosses `INT_MAX` and wraps to 0)
- [x] 12    — `tests/private.rs::priv_cp_find` (multi-chunk buffers, absent tags, `end` at 0 / mid-buffer / full)
- [x] 13 — `tests/aborts.rs::abort13_bits_left_zero`
- [x] 14 — `tests/aborts.rs::abort14_would_overflow`
- [x] 15 — `tests/aborts.rs::abort15_consume_bits_count`
- [x] 16 — `tests/aborts.rs::abort16_decode_mismatch`
- [x] 17 — `tests/aborts.rs::abort17_build_len_ge_16`
- [x] 18-21 — row 20: `tests/aborts.rs::abort20_read_bits_too_many` (both `cp_dist_extra_bits[0] = 33` and `cp_len_extra_bits[0] = 64` via the exported mutable tables). Rows 18/19/21 are proven non-triggering by the whole Phase B corpus (41 configuration rows, thousands of randomized streams) running abort-free in both libraries, and by `tests/risky.rs` which drives the negative-`first_bytes` state in a child process and requires identical stderr.
- [x] 22 — `tests/aborts.rs::abort22_bits_left_zero_null_input` (+ `abort13_bits_left_zero`, `abort13b_bits_left_zero_align3`)
- [x] 23 — `tests/aborts.rs::abort23_in_bytes_negative`
- [x] 24 — `tests/errors.rs::err24_out_bytes_zero`
- [x] 25 — `tests/errors.rs::err25_out_bytes_negative`
- [x] 26 — `tests/errors.rs::err26_out_one_byte_short`
- [x] 27 — `tests/errors.rs::err27_convert_pix_bad_bpp`
- [x] 28 — `tests/errors.rs::err28_convert_pix_nonpositive_dims`
- [x] 29 — `tests/errors.rs::err29_convert_pix_zero_width`
- [x] 30 — `tests/errors.rs::err30_error_reason_initially_null`
- [x] 31 — `tests/errors.rs::err31_error_reason_not_cleared_on_success`

## Additional error/edge configurations found while testing

| # | function | trigger | expected C result | test |
|---|----------|---------|-------------------|------|
| 32 | `cp_stored` (L187) | `LEN` GREATER than the remaining input: accepted (L179 only bounds `LEN` from below), then `memcpy` over-reads the input AND over-writes past `out + out_bytes` — `cp_stored` has no `out_end` check at all | returns 1, `out` filled from the wrong source bytes | `tests/valid.rs::row40_stored_len_beyond_remaining_input` |
| 33 | `cp_stored` / `cp_ptr` | `(in_bytes - first_bytes) % 4 != 0`: `cp_peak_bits` adds `s->bits_left` (not the *unbuffered* bit count) when folding in `s->final_word`, so `s->count` is too large by the old `count` and `cp_ptr()` returns a byte address that is too low — the stored payload is copied from the wrong offset | returns 1 with shifted output | `tests/valid.rs::row20/21/22` (payload equality asserted only for the word-aligned shapes; C/Rust equality asserted always) |
| 34 | `cp_inflate` | `in_bytes < first_bytes` (misaligned input shorter than its alignment padding): `word_count` and `last_bytes` come from a negative numerator, the pre-load loop reads past `in + in_bytes`, and the final-word loop reads *before* `in`. Some `(align, in_bytes)` pairs return, others `assert` | identical outcome in both libraries | `tests/risky.rs::row41_in_bytes_smaller_than_first_bytes` |
| 35 | `cp_read_bits` (L109 vs L119) | stored-block header truncated to 3 bytes: the alignment discard plus LEN/NLEN exhaust `bits_left` so the `s->bits_left > 0` assert wins instead of the `count` assert | `SIGABRT` at L119 | `tests/aborts.rs::abort15_consume_bits_count` |

## E. Non-terminating input (recorded, cannot be asserted on)

| # | function | trigger | C behaviour |
|---|----------|---------|-------------|
| 36 | `cp_find` (L405) | a chunk whose length field is exactly `0xFFFFFFF4`, so `len + 12` wraps to `0` and `png->p += len + 12` does not advance | **infinite loop** — `while (png->p < png->end)` never makes progress. The Rust reproduces it exactly (same wrapping add, same zero advance), so both hang; a test would hang too, so this row is documented instead of asserted. `cp_chunk`, which has no loop, IS tested with the same length field (`tests/private.rs::priv_cp_chunk`). |
| 37 | `cp_block` (L295) | a corrupt tree that makes `length` negative, so `memset(dst, *src, length)` gets `(size_t)(-n)` | both attempt an ~2^64-byte fill and fault; UB, not asserted |
