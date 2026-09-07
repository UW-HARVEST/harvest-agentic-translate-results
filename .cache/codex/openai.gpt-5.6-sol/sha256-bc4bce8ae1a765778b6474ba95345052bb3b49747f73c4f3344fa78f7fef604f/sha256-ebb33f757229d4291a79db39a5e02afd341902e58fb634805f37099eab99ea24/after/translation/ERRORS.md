# Error-surface table

Each row below is derived from an explicit C rejection or assertion in
`c_src/src/lib.c`. An assertion result means the C process terminates with
`SIGABRT` in this build.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `cp_ptr` via `pinflate` | `s->bits_left & 7` is nonzero when a stored block asks for its byte pointer | assertion failure (`SIGABRT`) | [x] |
| 2 | `cp_peak_bits` via `pinflate` | after loading a full word, `s->word_index > s->word_count` | assertion failure (`SIGABRT`) | [x] |
| 3 | `cp_consume_bits` via `pinflate` | requested bit count is greater than `s->count` | assertion failure (`SIGABRT`) | [x] |
| 4 | `cp_read_bits` via `pinflate` | requested bit count is greater than 32 | assertion failure (`SIGABRT`) | [x] |
| 5 | `cp_read_bits` via `pinflate` | requested bit count is negative | assertion failure (`SIGABRT`) | [x] |
| 6 | `cp_read_bits` via `pinflate` | `s->bits_left <= 0` before a read | assertion failure (`SIGABRT`) | [x] |
| 7 | `cp_read_bits` via `pinflate` | `s->count > 64` before a read | assertion failure (`SIGABRT`) | [x] |
| 8 | `cp_read_bits` via `pinflate` | `(s->bits_left + s->count) - num_bits_to_read < 0` | assertion failure (`SIGABRT`) | [x] |
| 9 | `cp_build` via `pinflate` | an input Huffman code length is 16 or greater | assertion failure (`SIGABRT`) | [x] |
| 10 | `cp_decode` via `pinflate` | the available prefix does not match the selected Huffman tree key | assertion failure (`SIGABRT`) | [x] |
| 11 | `cp_stored` via `pinflate` | stored-block `LEN != (uint16_t)~NLEN` | return `0`; reason `Failed to find LEN and NLEN as complements within stored (uncompressed) stream.` | [x] |
| 12 | `cp_stored` via `pinflate` | after reading `LEN`/`NLEN`, `s->bits_left / 8 > LEN` | return `0`; reason `Stored block extends beyond end of input stream.` | [x] |
| 13 | `cp_block` via `pinflate` | literal output would make `s->out + 1 > s->out_end` | return `0`; reason `Attempted to overwrite out buffer while outputting a symbol.` | [x] |
| 14 | `cp_block` via `pinflate` | a length/distance pair has `s->out - backwards_distance < s->begin` | return `0`; reason `Attempted to write before out buffer (invalid backwards distance).` | [x] |
| 15 | `cp_block` via `pinflate` | a length/distance pair has `s->out + length > s->out_end` | return `0`; reason `Attempted to overwrite out buffer while outputting a string.` | [x] |
| 16 | `pinflate` | three-bit block header has `BTYPE == 3` | return `0`; reason `Detected unknown block type within input stream.` | [x] |

Generic FFI boundaries additionally exercised in Phase C, even though the C
API contains no explicit guard for them: null input/output pointers, zero and
negative lengths, oversized signed lengths, and output lengths one byte below
the required size. The API has no enum parameter.

Rows 1–10 are internal invariant guards behind the sole exported function.
`phase_c_crash_and_generic_boundary_rows` compares process termination for the
externally reachable malformed-input cases, and
`translated_assertion_surface_is_release_active` verifies that all ten
translated guards remain active in the release library. Rows 11–16 are checked
individually by `phase_c_explicit_error_rows`, including exact return values and
error strings.
