# Valid configuration surface

Public entry points: `hdr_compare` (the only declaration in `include/lib.h`).
`hdr_valid` is `static` and is covered through `hdr_compare`.

The rows are the source-derived, short-circuit-pruned cross-product of:

- the two accepted `h2[1]` header patterns: high nibble `0xf`, or
  `(h2[1] & 0xfe) == 0xe2`;
- the two valid `h2[2]` upper-nibble shapes: zero or `1..=14`;
- the four outcomes distinguished after validation: byte-1 mismatch,
  byte-1 match then mode mismatch, both match then zero-class mismatch, and
  complete match.

Every row randomizes ignored bits and all values within the stated shape,
including all three accepted layer/mode field values where applicable.

| # | entry point(s) | configuration (options set + input shape) | Verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `hdr_compare` | `h2[1]` high nibble `0xf`; `h2[2]` high nibble zero; masked byte-1 mismatch | [x] |
| 2 | `hdr_compare` | `h2[1]` high nibble `0xf`; `h2[2]` high nibble nonzero `1..=14`; masked byte-1 mismatch | [x] |
| 3 | `hdr_compare` | `h2[1] & 0xfe == 0xe2`; `h2[2]` high nibble zero; masked byte-1 mismatch | [x] |
| 4 | `hdr_compare` | `h2[1] & 0xfe == 0xe2`; `h2[2]` high nibble nonzero `1..=14`; masked byte-1 mismatch | [x] |
| 5 | `hdr_compare` | `h2[1]` high nibble `0xf`; `h2[2]` high nibble zero; byte-1 match, mode-field mismatch | [x] |
| 6 | `hdr_compare` | `h2[1]` high nibble `0xf`; `h2[2]` high nibble nonzero `1..=14`; byte-1 match, mode-field mismatch | [x] |
| 7 | `hdr_compare` | `h2[1] & 0xfe == 0xe2`; `h2[2]` high nibble zero; byte-1 match, mode-field mismatch | [x] |
| 8 | `hdr_compare` | `h2[1] & 0xfe == 0xe2`; `h2[2]` high nibble nonzero `1..=14`; byte-1 match, mode-field mismatch | [x] |
| 9 | `hdr_compare` | `h2[1]` high nibble `0xf`; `h2[2]` high nibble zero; byte-1 and mode match, zero/nonzero class mismatch | [x] |
| 10 | `hdr_compare` | `h2[1]` high nibble `0xf`; `h2[2]` high nibble nonzero `1..=14`; byte-1 and mode match, zero/nonzero class mismatch | [x] |
| 11 | `hdr_compare` | `h2[1] & 0xfe == 0xe2`; `h2[2]` high nibble zero; byte-1 and mode match, zero/nonzero class mismatch | [x] |
| 12 | `hdr_compare` | `h2[1] & 0xfe == 0xe2`; `h2[2]` high nibble nonzero `1..=14`; byte-1 and mode match, zero/nonzero class mismatch | [x] |
| 13 | `hdr_compare` | `h2[1]` high nibble `0xf`; `h2[2]` high nibble zero; all compared fields match | [x] |
| 14 | `hdr_compare` | `h2[1]` high nibble `0xf`; `h2[2]` high nibble nonzero `1..=14`; all compared fields match | [x] |
| 15 | `hdr_compare` | `h2[1] & 0xfe == 0xe2`; `h2[2]` high nibble zero; all compared fields match | [x] |
| 16 | `hdr_compare` | `h2[1] & 0xfe == 0xe2`; `h2[2]` high nibble nonzero `1..=14`; all compared fields match | [x] |
