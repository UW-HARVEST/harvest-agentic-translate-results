# Error surface

Derived mechanically from every `assert`, `cp_error_reason` assignment,
error `goto`, and public length/pointer boundary in `../c_src/src/lib.c`.
The only public function is `pinflate`; rows naming a static helper identify
the helper reached through `pinflate`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| [x] 1 | `cp_stored` via `pinflate` | Stored-block `LEN != (uint16_t)~NLEN`. | Return `0`; `cp_error_reason` is `Failed to find LEN and NLEN as complements within stored (uncompressed) stream.` |
| [x] 2 | `cp_stored` via `pinflate` | After the stored header, `s->bits_left / 8 > LEN` (including a short stored block followed by trailing bytes or another block). | Return `0`; `cp_error_reason` is `Stored block extends beyond end of input stream.` |
| [x] 3 | `cp_block` via `pinflate` | A literal is decoded while `s->out + 1 > s->out_end`. | Return `0`; `cp_error_reason` is `Attempted to overwrite out buffer while outputting a symbol.` |
| [x] 4 | `cp_block` via `pinflate` | A match is decoded with `s->out - backwards_distance < s->begin`. | Return `0`; `cp_error_reason` is `Attempted to write before out buffer (invalid backwards distance).` |
| [x] 5 | `cp_block` via `pinflate` | A match is decoded with `s->out + length > s->out_end`. | Return `0`; `cp_error_reason` is `Attempted to overwrite out buffer while outputting a string.` |
| [x] 6 | `pinflate` | The three-bit block header has `BTYPE == 3`. | Return `0`; `cp_error_reason` is `Detected unknown block type within input stream.` |
| [x] 7 | `cp_ptr` via `pinflate` | Internal stored-block invariant `(s->bits_left & 7) != 0`. The public parser consumes `s->count & 7` before this call, so no byte input can reach it without state corruption. | `assert` abort (`SIGABRT`) in this C build. |
| [x] 8 | `cp_peak_bits` via `pinflate` | Internal refill invariant `s->word_index > s->word_count` after increment. The guarded increment starts only when `word_index < word_count`, so public input cannot violate it without state corruption. | `assert` abort (`SIGABRT`) in this C build. |
| [x] 9 | `cp_consume_bits` via `pinflate` | `s->count < num_bits_to_read`. A truncated stream can exhaust buffered bits before a requested field or Huffman code is consumed. | `assert` abort (`SIGABRT`) in this C build. |
| [x] 10 | `cp_read_bits` via `pinflate` | `num_bits_to_read > 32`. All call sites use constants/table values in `0..=16`, so no public byte input can reach this condition without state/table corruption. | `assert` abort (`SIGABRT`) in this C build. |
| [x] 11 | `cp_read_bits` via `pinflate` | `num_bits_to_read < 0`. All call sites use nonnegative constants and unsigned table values, so no public byte input can reach this condition without state/table corruption. | `assert` abort (`SIGABRT`) in this C build. |
| [x] 12 | `cp_read_bits` via `pinflate` | `s->bits_left <= 0` when another field or symbol is requested, including zero-length or truncated input. | `assert` abort (`SIGABRT`) in this C build. |
| [x] 13 | `cp_read_bits` via `pinflate` | `s->count > 64`. Initial buffering is at most 24 bits and refill adds one 32-bit word only when needed, so public input cannot violate this without state corruption. | `assert` abort (`SIGABRT`) in this C build. |
| [x] 14 | `cp_read_bits` via `pinflate` | `(s->bits_left + s->count) - num_bits_to_read < 0`, meaning the requested field extends beyond all remaining input bits. | `assert` abort (`SIGABRT`) in this C build. |
| [x] 15 | `cp_build` via `pinflate` | A nonzero Huffman code length is `>= 16`. Fixed lengths are at most 9 and dynamic lengths are read from three bits (`0..=7`), so no public byte input can reach this without mutating exported tables or corrupting state. | `assert` abort (`SIGABRT`) in this C build. |
| [x] 16 | `cp_decode` via `pinflate` | The selected Huffman entry does not share the searched prefix: `(search >> len) != (key >> len)`, as can occur for an incomplete/malformed code tree. | `assert` abort (`SIGABRT`) in this C build. |
| [x] 17 | `pinflate` | `in == NULL` with `in_bytes > 0`. There is no null check; input is dereferenced. | Process fault (`SIGSEGV` on the test platform), not an error return. |
| [x] 18 | `pinflate` | `out == NULL` with a valid stream that emits at least one byte. There is no null check; output is written. | Process fault (`SIGSEGV` on the test platform), not an error return. |
| [x] 19 | `pinflate` | `in_bytes == 0`. There is no length check; the first one-bit read violates `s->bits_left > 0`. | `assert` abort (`SIGABRT`) in this C build. |
| [x] 20 | `pinflate` | `out_bytes == 0` with a valid nonempty literal stream. | Same return and exact reason as row 3. |
| [x] 21 | `pinflate` | `out_bytes` is one byte below the required match output size. | Same return and exact reason as row 5. |
| [x] 22 | `pinflate` | The block-type field is one step above the valid range `0..=2` (`BTYPE == 3`); this API has no C enum parameters. | Same return and exact reason as row 6. |

The C API contains no explicit null checks, no enum-typed parameters, and no
documented positive maximum for either signed `int` length. Negative lengths
and lengths that overflow `in_bytes * 8` invoke C undefined behavior, so they
do not define a stable C result that a translation can compare.
