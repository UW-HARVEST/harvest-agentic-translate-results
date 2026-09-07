# Error surface

Mechanically derived from every `assert`, explicit rejection condition, and
error return in `c_src/src/lib.c`. Rows 1-8 are externally observable return
paths. Rows 9-18 record the internal assertions reached through `cp_inflate`;
several are invariants whose operands are wholly selected by internal code, so
no defined FFI input can independently violate them. Malformed/truncated input
tests cover the externally reachable assertion classes without invoking C
undefined behavior deliberately.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `cp_stored` via `cp_inflate` | stored-block `LEN != (uint16_t)~NLEN` | return `0`; `cp_error_reason` = complement message |
| 2 | `cp_stored` via `cp_inflate` | after the stored header, `s->bits_left / 8 > LEN` | return `0`; `cp_error_reason` = stored-end message |
| 3 | `cp_block` via `cp_inflate` | literal symbol with `s->out + 1 > s->out_end` | return `0`; `cp_error_reason` = symbol-output message |
| 4 | `cp_block` via `cp_inflate` | length/distance pair with `s->out - backwards_distance < s->begin` | return `0`; `cp_error_reason` = backwards-distance message |
| 5 | `cp_block` via `cp_inflate` | length/distance pair with `s->out + length > s->out_end` | return `0`; `cp_error_reason` = string-output message |
| 6 | `cp_inflate` | block header has `btype == 3` | return `0`; `cp_error_reason` = unknown-block message |
| 7 | `unfilter` | `h > 0` and first scanline filter byte is outside `0..=4` | return `0` after no scanline decoding |
| 8 | `unfilter` | `h > 1` and any later scanline filter byte is outside `0..=4` | return `0` after earlier scanlines were decoded |
| 9 | `cp_ptr` via `cp_inflate` | internal stored-data pointer requested while `(s->bits_left & 7) != 0` | assertion failure (`SIGABRT`) |
| 10 | `cp_peak_bits` via `cp_inflate` | internal word load makes `s->word_index > s->word_count` | assertion failure (`SIGABRT`) |
| 11 | `cp_consume_bits` via `cp_inflate` | internal decoder asks to consume more than `s->count` bits | assertion failure (`SIGABRT`) |
| 12 | `cp_read_bits` via `cp_inflate` | internal caller requests more than 32 bits | assertion failure (`SIGABRT`) |
| 13 | `cp_read_bits` via `cp_inflate` | internal caller requests a negative bit count | assertion failure (`SIGABRT`) |
| 14 | `cp_read_bits` via `cp_inflate` | truncated/malformed input reaches a read with `s->bits_left <= 0` | assertion failure (`SIGABRT`) |
| 15 | `cp_read_bits` via `cp_inflate` | internal bit accumulator has `s->count > 64` | assertion failure (`SIGABRT`) |
| 16 | `cp_read_bits` via `cp_inflate` | requested read satisfies `(s->bits_left + s->count) - num_bits < 0` | assertion failure (`SIGABRT`) |
| 17 | `cp_build` via `cp_inflate` | an internally decoded nonzero Huffman code length is `>= 16` | assertion failure (`SIGABRT`) |
| 18 | `cp_decode` via `cp_inflate` | malformed Huffman bits do not match the selected canonical tree key | assertion failure (`SIGABRT`) |

Boundary cases required in addition to the C-authored rejection branches:

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 19 | `unfilter` | `h <= 0`, including `raw == NULL` | return `1`; pointer is not dereferenced |
| 20 | `unfilter` | filter values `5` and `255` (out-of-range byte/enum-like discriminator) | return `0` |
| 21 | `unfilter` | zero row width with `h == 1`, or `bpp == 0` with valid filter bytes | return `1` |
| 22 | `cp_inflate` | zero output length with an empty valid stream | return `1` |
| 23 | `cp_inflate` | zero output length with a stream producing a literal | return `0` with symbol-output reason |
| 24 | `cp_inflate` | zero input length | assertion failure (`SIGABRT`) |
| 25 | `cp_inflate` | `btype == 3`, the one-step-past-valid block-type value | return `0` with unknown-block reason |

Null nonzero-length input/output pointers and signed integer overflow in
`in_bytes * 8`, `w * bpp`, or pointer arithmetic are undefined behavior in the
C ground truth, not defined rejection results. They are excluded from
byte/result parity rather than assigning invented semantics.

Phase C status:

- [x] Rows 1-8: exact return value, error string, and buffer mutation compared.
- [x] Rows 9-18: C assertion predicates are mirrored in Rust; externally
  reachable assertion classes are compared in isolated malformed-input
  subprocesses.
- [x] Rows 19-25: exact boundary result or process outcome compared.
