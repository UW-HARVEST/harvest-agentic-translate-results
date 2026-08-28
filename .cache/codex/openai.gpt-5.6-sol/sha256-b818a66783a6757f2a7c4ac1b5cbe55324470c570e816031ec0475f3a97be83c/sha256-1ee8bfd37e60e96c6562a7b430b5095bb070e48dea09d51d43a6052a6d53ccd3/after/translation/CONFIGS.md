# Configuration Surface

There is one public entry point, `hdr_compare`, no runtime options, and no
Cargo/C preprocessor features. Both inputs are pointers to buffers with at
least three readable bytes. `h1[0]` is ignored.

The C source distinguishes these valid-input axes:

- `h2[1]` sync form: `(h2[1] & 0xf0) == 0xf0` with nonzero layer bits, or
  `(h2[1] & 0xfe) == 0xe2`.
- `h2[2]` high nibble: zero, or nonzero and not `0xf`.
- `h2[2]` bits 3:2: one of the valid values 0, 1, or 2.
- Comparison path: masked byte-1 mismatch; masked byte-1 match followed by
  bits-3:2 mismatch; both masked fields match followed by a zero/nonzero
  high-nibble-class mismatch; or all compared properties match.

The table is the meaningful cross-product of the two sync forms, two valid
high-nibble classes, and four comparison paths. Every row randomizes ignored
bits and values within the stated classes.

| # | entry point(s) | configuration (options set + input shape) | tested |
|---:|---|---|:---:|
| 1 | `hdr_compare` | `0xf?` sync; zero high nibble; masked byte-1 mismatch | [x] |
| 2 | `hdr_compare` | `0xf?` sync; zero high nibble; byte-1 match, bits-3:2 mismatch | [x] |
| 3 | `hdr_compare` | `0xf?` sync; zero high nibble; masked fields match, high-nibble class mismatch | [x] |
| 4 | `hdr_compare` | `0xf?` sync; zero high nibble; all compared properties match | [x] |
| 5 | `hdr_compare` | `0xf?` sync; nonzero non-`0xf` high nibble; masked byte-1 mismatch | [x] |
| 6 | `hdr_compare` | `0xf?` sync; nonzero non-`0xf` high nibble; byte-1 match, bits-3:2 mismatch | [x] |
| 7 | `hdr_compare` | `0xf?` sync; nonzero non-`0xf` high nibble; masked fields match, high-nibble class mismatch | [x] |
| 8 | `hdr_compare` | `0xf?` sync; nonzero non-`0xf` high nibble; all compared properties match | [x] |
| 9 | `hdr_compare` | `0xe2/0xe3` sync; zero high nibble; masked byte-1 mismatch | [x] |
| 10 | `hdr_compare` | `0xe2/0xe3` sync; zero high nibble; byte-1 match, bits-3:2 mismatch | [x] |
| 11 | `hdr_compare` | `0xe2/0xe3` sync; zero high nibble; masked fields match, high-nibble class mismatch | [x] |
| 12 | `hdr_compare` | `0xe2/0xe3` sync; zero high nibble; all compared properties match | [x] |
| 13 | `hdr_compare` | `0xe2/0xe3` sync; nonzero non-`0xf` high nibble; masked byte-1 mismatch | [x] |
| 14 | `hdr_compare` | `0xe2/0xe3` sync; nonzero non-`0xf` high nibble; byte-1 match, bits-3:2 mismatch | [x] |
| 15 | `hdr_compare` | `0xe2/0xe3` sync; nonzero non-`0xf` high nibble; masked fields match, high-nibble class mismatch | [x] |
| 16 | `hdr_compare` | `0xe2/0xe3` sync; nonzero non-`0xf` high nibble; all compared properties match | [x] |

Feature/build combinations:

| # | Cargo invocation surface | C preprocessor surface | tested |
|---:|---|---|:---:|
| 1 | default features (none declared) | no conditional compilation | [x] |
| 2 | `--no-default-features` (equivalent, verified independently) | no conditional compilation | [x] |
