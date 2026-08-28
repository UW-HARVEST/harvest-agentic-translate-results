# Error surface

This table comes from all `assert`, error assignment, null/sentinel return,
range check, and bounds-check sites in `src/lib.c`. Rows marked "invariant"
are mechanically present assertions in static helpers, but their trigger
cannot be supplied through either exported function because the caller does
not control `cp_state_t` or Huffman code lengths.

| # | function | trigger (the exact invalid input/condition) | expected C result | Status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `cp_ptr` via `cp_inflate` | `s->bits_left & 7` is nonzero | assertion failure; internal invariant after stored-block byte alignment | [x] invariant audit |
| 2 | `cp_peak_bits` via `cp_inflate` | loading a word makes `s->word_index > s->word_count` | assertion failure; internal word-index invariant | [x] invariant audit |
| 3 | `cp_consume_bits` via `cp_inflate` | `s->count < num_bits_to_read` | assertion failure | [x] fatal differential |
| 4 | `cp_read_bits` via `cp_inflate` | `num_bits_to_read > 32` | assertion failure; no exported path requests more than 16 bits | [x] invariant audit |
| 5 | `cp_read_bits` via `cp_inflate` | `num_bits_to_read < 0` | assertion failure; no exported path computes a negative request | [x] invariant audit |
| 6 | `cp_read_bits` via `cp_inflate` | `s->bits_left <= 0`, including zero-length input | assertion failure | [x] fatal differential |
| 7 | `cp_read_bits` via `cp_inflate` | `s->count > 64` | assertion failure; internal bit-buffer invariant | [x] invariant audit |
| 8 | `cp_read_bits` via `cp_inflate` | `(s->bits_left + s->count) - num_bits_to_read < 0`, including truncated input | assertion failure | [x] invariant audit |
| 9 | `cp_build` via `cp_inflate` | a nonzero Huffman code length is at least 16 | assertion failure; fixed lengths are at most 9 and dynamic lengths are 3-bit values | [x] invariant audit |
| 10 | `cp_stored` via `cp_inflate` | stored `LEN != (uint16_t)~NLEN` | return `0`; reason `Failed to find LEN and NLEN as complements within stored (uncompressed) stream.` | [x] return differential |
| 11 | `cp_stored` via `cp_inflate` | after the stored header, `s->bits_left / 8 > LEN` | return `0`; reason `Stored block extends beyond end of input stream.` | [x] return differential |
| 12 | `cp_decode` via `cp_inflate` | decoded search prefix differs from the selected Huffman key prefix | assertion failure on an invalid Huffman stream | [x] fatal differential |
| 13 | `cp_block` via `cp_inflate` | literal requires `s->out + 1 > s->out_end`, including zero output length | return `0`; reason `Attempted to overwrite out buffer while outputting a symbol.` | [x] return differential |
| 14 | `cp_block` via `cp_inflate` | match has `s->out - backwards_distance < s->begin` | return `0`; reason `Attempted to write before out buffer (invalid backwards distance).` | [x] return differential |
| 15 | `cp_block` via `cp_inflate` | match requires `s->out + length > s->out_end` | return `0`; reason `Attempted to overwrite out buffer while outputting a string.` | [x] return differential |
| 16 | `cp_inflate` | DEFLATE `BTYPE == 3` | return `0`; reason `Detected unknown block type within input stream.` | [x] return differential |
| 17 | `cp_chunk` | chunk type differs, `len < minlen`, or `png->p + len + 12 > png->end` | return null; static helper is not reachable from an exported symbol | [x] static reachability audit |
| 18 | `cp_find` | no matching in-bounds chunk of at least `minlen` exists before `png->end` | return null; static helper is not reachable from an exported symbol | [x] static reachability audit |
| 19 | `cp_unfilter` | first scanline filter byte is outside `0..=4` | return `0`; static helper is not reachable from an exported symbol | [x] static reachability audit |
| 20 | `cp_unfilter` | any later scanline filter byte is outside `0..=4` | return `0`; static helper is not reachable from an exported symbol | [x] static reachability audit |

`convert_pix` has no error return. Unsupported `bpp`, zero/negative dimensions,
and zero-iteration null pointers are valid no-op configurations and are listed
in `CONFIGS.md`. A null pointer with a positive dereference count and signed
length arithmetic overflow invoke C undefined behavior, so the C source does
not define a rejection result for those inputs.
