# Error surface

Each row is derived from an explicit C rejection or assertion in
`../c_src/src/lib.c`. “Internal-only” means the condition cannot be directly
constructed through either exported function because the relevant function is
`static` or the assertion protects an invariant imposed by all exported call
paths.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `cp_ptr` | `s->bits_left & 7 != 0` | `assert` abort; internal invariant after stored-block byte alignment | [x] |
| 2 | `cp_peak_bits` | loading a word makes `s->word_index > s->word_count` | `assert` abort; internal counter invariant | [x] |
| 3 | `cp_consume_bits` | `s->count < num_bits_to_read` | `assert` abort | [x] |
| 4 | `cp_read_bits` | `num_bits_to_read > 32` | `assert` abort; no exported path requests more than 32 | [x] |
| 5 | `cp_read_bits` | `num_bits_to_read < 0` | `assert` abort; no exported path requests a negative count | [x] |
| 6 | `cp_read_bits` | `s->bits_left <= 0` | `assert` abort (for example zero-byte/truncated input) | [x] |
| 7 | `cp_read_bits` | `s->count > 64` | `assert` abort; internal bit-buffer invariant | [x] |
| 8 | `cp_read_bits` / `cp_would_overflow` | `(s->bits_left + s->count) - num_bits_to_read < 0` | `assert` abort on truncated input | [x] |
| 9 | `cp_build` | a code length is `>= 16` | `assert` abort; internal-only because fixed lengths and 3-bit dynamic lengths are at most 15 | [x] |
| 10 | `cp_decode` | decoded search prefix does not equal the selected tree prefix | `assert` abort on malformed Huffman input | [x] |
| 11 | `cp_stored` | `LEN != (uint16_t)~NLEN` | return `0`; reason is “Failed to find LEN and NLEN as complements within stored (uncompressed) stream.” | [x] |
| 12 | `cp_stored` | `s->bits_left / 8 > LEN` (the exact, unusual C comparison) | return `0`; reason is “Stored block extends beyond end of input stream.” | [x] |
| 13 | `cp_block` | a literal is decoded when `s->out + 1 > s->out_end` | return `0`; reason is “Attempted to overwrite out buffer while outputting a symbol.” | [x] |
| 14 | `cp_block` | a length/distance pair has `s->out - backwards_distance < s->begin` | return `0`; reason is “Attempted to write before out buffer (invalid backwards distance).” | [x] |
| 15 | `cp_block` | a length/distance pair has `s->out + length > s->out_end` | return `0`; reason is “Attempted to overwrite out buffer while outputting a string.” | [x] |
| 16 | `cp_inflate` | block type bits are `3` | return `0`; reason is “Detected unknown block type within input stream.” | [x] |
| 17 | `cp_unfilter` | first row filter byte is not in `0..=4` | return `0`; internal-only static function, unreachable from the exported ABI | [x] |
| 18 | `cp_unfilter` | any later row filter byte is not in `0..=4` | return `0`; internal-only static function, unreachable from the exported ABI | [x] |

Generic FFI boundary cases (in addition to the mechanically derived rows):

| # | function | boundary | expected C result | verified |
|---|----------|----------|-------------------|----------|
| G1 | `convert_pix` | null `src` and `dst` with zero width or height | returns without dereference | [x] |
| G2 | `convert_pix` | null `src` or `dst` with positive dimensions | process fault/abort | [x] |
| G3 | `convert_pix` | zero and negative width/height | returns without touching destination | [x] |
| G4 | `convert_pix` | unsupported `bpp` (`0`, `5`, `-1`, `INT_MAX`) | consumes rows/pixels according to C pointer arithmetic but performs no pixel write | [x] |
| G5 | `cp_inflate` | null input with zero length | assertion abort | [x] |
| G6 | `cp_inflate` | null output with a valid zero-output stream | return `1` without output dereference | [x] |
| G7 | `cp_inflate` | zero output length with nonempty decoded output | same exact `0` rejection and reason as C | [x] |
| G8 | `cp_inflate` | input/output lengths one step below zero (`-1`) | same process-level rejection as C | [x] |
| G9 | public API | out-of-range enum | not applicable: the C ABI declares no enum parameters | [x] |

Verification notes:

- Rows 3, 6, and 8 are covered by zero/truncated input subprocess probes; row
  10 is covered by an empty dynamic-Huffman tree that triggers the exact
  `cp_decode` assertion in both libraries.
- Rows 1, 2, 4, 5, 7, and 9 are invariants that exported `cp_inflate` cannot
  violate: alignment is forced before `cp_ptr`, requested bit counts are fixed
  and nonnegative, the bit buffer is bounded, and source code lengths cannot
  reach 16. The test suite confirms these helpers are not exported by either
  library.
- Rows 17 and 18 belong to a dead `static` helper with no caller and no dynamic
  symbol in the C library; the test suite confirms it is inaccessible in both
  dynamic symbol surfaces.
