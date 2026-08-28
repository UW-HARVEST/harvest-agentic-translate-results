# Error surface

This table follows every explicit rejection, error return, range check, and
assertion in `c_src/src/lib.c`. Static helper rows identify the exported entry
point through which they are reachable. Assertions describe the C build used
for verification, where `NDEBUG` is not defined.

| # | function | trigger (the exact invalid input/condition) | expected C result | Covered |
|---|----------|---------------------------------------------|-------------------|---------|
| 1 | `cp_ptr` via `cp_inflate` | `s->bits_left & 7` is nonzero when a stored block asks for its byte pointer | assertion failure (`SIGABRT`) | [x] |
| 2 | `cp_peak_bits` via `cp_inflate` | loading a full word increments `word_index` above `word_count` | assertion failure (`SIGABRT`) | [x] |
| 3 | `cp_consume_bits` via `cp_inflate` | `s->count < num_bits_to_read` | assertion failure (`SIGABRT`) | [x] |
| 4 | `cp_read_bits` via `cp_inflate` | `num_bits_to_read > 32` | assertion failure (`SIGABRT`) | [x] |
| 5 | `cp_read_bits` via `cp_inflate` | `num_bits_to_read < 0` | assertion failure (`SIGABRT`) | [x] |
| 6 | `cp_read_bits` via `cp_inflate` | `s->bits_left <= 0` before a read | assertion failure (`SIGABRT`) | [x] |
| 7 | `cp_read_bits` via `cp_inflate` | `s->count > 64` before a read | assertion failure (`SIGABRT`) | [x] |
| 8 | `cp_read_bits` via `cp_inflate` | `(s->bits_left + s->count) - num_bits_to_read < 0` | assertion failure (`SIGABRT`) | [x] |
| 9 | `cp_build` via `cp_inflate` | a nonzero Huffman code length is `>= 16` | assertion failure (`SIGABRT`) | [x] |
| 10 | `cp_decode` via `cp_inflate` | the selected Huffman key prefix does not equal the input prefix | assertion failure (`SIGABRT`) | [x] |
| 11 | `cp_stored` via `cp_inflate` | `LEN != (uint16_t)~NLEN` | returns `0`; `cp_error_reason` is `Failed to find LEN and NLEN as complements within stored (uncompressed) stream.` | [x] |
| 12 | `cp_stored` via `cp_inflate` | `s->bits_left / 8 > LEN` (the comparison is intentionally recorded exactly as C implements it) | returns `0`; `cp_error_reason` is `Stored block extends beyond end of input stream.` | [x] |
| 13 | `cp_block` via `cp_inflate` | a literal needs one byte but `s->out + 1 > s->out_end` | returns `0`; `cp_error_reason` is `Attempted to overwrite out buffer while outputting a symbol.` | [x] |
| 14 | `cp_block` via `cp_inflate` | a length/distance pair has `s->out - backwards_distance < s->begin` | returns `0`; `cp_error_reason` is `Attempted to write before out buffer (invalid backwards distance).` | [x] |
| 15 | `cp_block` via `cp_inflate` | a length/distance pair has `s->out + length > s->out_end` | returns `0`; `cp_error_reason` is `Attempted to overwrite out buffer while outputting a string.` | [x] |
| 16 | `cp_inflate` | the two-bit block type is `3` | returns `0`; `cp_error_reason` is `Detected unknown block type within input stream.` | [x] |
| 17 | `cp_chunk` (static, not reachable from an exported entry point) | chunk type differs, `len < minlen`, or `png->p + len + 12 > png->end` | returns null | [x] |
| 18 | `cp_find` (static, not reachable from an exported entry point) | no chunk has matching type, `len >= minlen`, and an end pointer within `png->end` before iteration ends | returns null | [x] |
| 19 | `unfilter` | first-row filter byte is outside `0..=4` when `h > 0` | returns `0` | [x] |
| 20 | `unfilter` | any later-row filter byte is outside `0..=4` when `h > 1` | returns `0` after earlier rows may have been modified | [x] |

Rows 17-18 cannot be exercised through the shared-library ABI because both
functions have internal linkage and no exported caller. Their source-derived
behavior is retained here so the error inventory is complete; their checks
mean source-inventory coverage, not an ABI differential call. Rows 1-10 are
mirrored by `internal_assertion_surface_is_mirrored`; reachable assertion
failures are additionally compared in isolated processes by
`process_level_error_boundaries_match`. Rows 11-16 and 19-20 are exercised by
`explicit_error_returns_match`.
