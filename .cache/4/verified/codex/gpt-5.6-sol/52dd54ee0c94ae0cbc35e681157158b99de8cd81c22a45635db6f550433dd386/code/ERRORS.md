# Error Surface

This table is mechanically derived from every `assert`, explicit failure
return, null/sentinel return, and unsupported public selector in
`c_src/src/lib.c`. Conditions marked **internal-only** occur in a `static`
helper that is not exported and is not called by either exported function.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `cp_stored` via `cp_inflate` | stored `LEN != (uint16_t)~NLEN` | `cp_inflate` returns `0`; `cp_error_reason` is `Failed to find LEN and NLEN as complements within stored (uncompressed) stream.` | [x] |
| 2 | `cp_stored` via `cp_inflate` | after the stored header, `s->bits_left / 8 > LEN` (the C comparison is intentionally this direction) | `cp_inflate` returns `0`; `cp_error_reason` is `Stored block extends beyond end of input stream.` | [x] |
| 3 | `cp_block` via `cp_inflate` | literal symbol with `s->out + 1 > s->out_end` | `cp_inflate` returns `0`; `cp_error_reason` is `Attempted to overwrite out buffer while outputting a symbol.` | [x] |
| 4 | `cp_block` via `cp_inflate` | length/distance symbol with `s->out - backwards_distance < s->begin` | `cp_inflate` returns `0`; `cp_error_reason` is `Attempted to write before out buffer (invalid backwards distance).` | [x] |
| 5 | `cp_block` via `cp_inflate` | length/distance symbol with `s->out + length > s->out_end` | `cp_inflate` returns `0`; `cp_error_reason` is `Attempted to overwrite out buffer while outputting a string.` | [x] |
| 6 | `cp_inflate` | block header has reserved `BTYPE == 3` | returns `0`; `cp_error_reason` is `Detected unknown block type within input stream.` | [x] |
| 7 | `cp_ptr` via `cp_inflate` | internal stored-block invariant `s->bits_left & 7 != 0` | process aborts at C `assert` | [x] |
| 8 | `cp_peak_bits` via `cp_inflate` | internal word-load invariant `s->word_index > s->word_count` | process aborts at C `assert` | [x] |
| 9 | `cp_consume_bits` via `cp_inflate` | decoder asks to consume more bits than `s->count` | process aborts at C `assert(s->count >= num_bits_to_read)` | [x] |
| 10 | `cp_read_bits` via `cp_inflate` | decoder requests more than 32 bits | process aborts at C `assert(num_bits_to_read <= 32)` | [x] |
| 11 | `cp_read_bits` via `cp_inflate` | decoder requests a negative bit count | process aborts at C `assert(num_bits_to_read >= 0)` | [x] |
| 12 | `cp_read_bits` via `cp_inflate` | decoder reads when `s->bits_left <= 0` | process aborts at C `assert(s->bits_left > 0)` | [x] |
| 13 | `cp_read_bits` via `cp_inflate` | internal accumulator invariant `s->count > 64` | process aborts at C `assert(s->count <= 64)` | [x] |
| 14 | `cp_read_bits` via `cp_inflate` | `(s->bits_left + s->count) - num_bits_to_read < 0` | process aborts at C `assert(!cp_would_overflow(...))` | [x] |
| 15 | `cp_build` via `cp_inflate` | a nonzero Huffman code length is at least 16 | process aborts at C `assert(len < 16)` | [x] |
| 16 | `cp_decode` via `cp_inflate` | malformed Huffman input does not match the selected tree key | process aborts at C prefix-match `assert` | [x] |
| 17 | `cp_unfilter` (**internal-only**) | first row filter byte is not in `0..=4` | returns `0` | [x] |
| 18 | `cp_unfilter` (**internal-only**) | any later row filter byte is not in `0..=4` | returns `0` | [x] |
| 19 | `cp_chunk` (**internal-only**) | chunk type differs from the requested four bytes | returns null | [x] |
| 20 | `cp_chunk` (**internal-only**) | chunk length is less than `minlen` | returns null | [x] |
| 21 | `cp_chunk` (**internal-only**) | `png->p + len + 12 > png->end` | returns null | [x] |
| 22 | `cp_find` (**internal-only**) | no requested chunk type is found before `png->end` | returns null | [x] |
| 23 | `cp_find` (**internal-only**) | matching chunk length is less than `minlen` and no later valid match exists | returns null | [x] |
| 24 | `cp_find` (**internal-only**) | matching chunk advances beyond `png->end` and no later valid match exists | returns null | [x] |
| 25 | `convert_pix` | `bpp` is any value other than `1`, `2`, `3`, or `4` while `w > 0` and `h > 0` | returns `void`; consumes rows/pixels but leaves destination bytes unchanged | [x] |
| 26 | `convert_pix` | null `src` and null `dst` with `h <= 0` | returns `void` without dereferencing either pointer | [x] |
| 27 | `convert_pix` | null `dst` with `w <= 0`, `h > 0`, and a valid nonnull `src` | returns `void` without dereferencing `dst` | [x] |
| 28 | `cp_inflate` | null `in` and/or null `out` with lengths that require access | C has no null check; behavior is undefined, so there is no C error code or sentinel to match | [x] |
| 29 | `cp_inflate` | zero input length | process aborts at `assert(s->bits_left > 0)` | [x] |
| 30 | `cp_inflate` | negative input length | process aborts at a bit-reader assertion before a defined return | [x] |
| 31 | `cp_inflate` | negative output length with a stream that emits a literal | returns `0` with the symbol-output error before writing | [x] |
| 32 | `cp_inflate` | output length one byte below a literal-only stream's required size | returns `0` with the symbol-output error | [x] |
| 33 | `cp_inflate` | output length one byte below a back-reference stream's required size | returns `0` with the string-output error | [x] |

Rows 7, 8, 10, 11, 13, and 15 are assertions over invariants whose violating
states cannot be constructed through the exported ABI without first invoking
undefined behavior. Rows 17-24 are not reachable from an exported C symbol.
They remain listed so the source-level rejection inventory is complete.
