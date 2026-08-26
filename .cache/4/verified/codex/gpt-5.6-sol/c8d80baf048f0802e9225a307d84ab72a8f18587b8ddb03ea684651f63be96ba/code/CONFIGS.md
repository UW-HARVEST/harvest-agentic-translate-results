# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` has no
options, conditional sources, or compile definitions. There is one valid
feature combination:

| # | Cargo features | C configuration | Check |
|---|----------------|-----------------|-------|
| 1 | Empty set (`--no-default-features`) | Default | [x] |

Verified with `cargo check --no-default-features`.

## Runtime Configurations

The public headers expose only `half2float(uint16_t)`. There are no runtime
options, modes, flags, pointers, lengths, byte-order choices, or composed
entry points. The source indexes its tables by the input's sign/exponent field
(`h >> 10`) and fraction field (`h & 0x03ff`). The resulting mechanically
distinct binary16 classes are:

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `half2float` | Positive zero: sign 0, exponent 0, fraction 0 (`0x0000`) | [x] |
| 2 | `half2float` | Positive subnormal: sign 0, exponent 0, fraction `1..0x3ff` | [x] |
| 3 | `half2float` | Positive normal: sign 0, exponent `1..30`, fraction `0..0x3ff` | [x] |
| 4 | `half2float` | Positive infinity: sign 0, exponent 31, fraction 0 (`0x7c00`) | [x] |
| 5 | `half2float` | Positive NaN: sign 0, exponent 31, fraction `1..0x3ff` | [x] |
| 6 | `half2float` | Negative zero: sign 1, exponent 0, fraction 0 (`0x8000`) | [x] |
| 7 | `half2float` | Negative subnormal: sign 1, exponent 0, fraction `1..0x3ff` | [x] |
| 8 | `half2float` | Negative normal: sign 1, exponent `1..30`, fraction `0..0x3ff` | [x] |
| 9 | `half2float` | Negative infinity: sign 1, exponent 31, fraction 0 (`0xfc00`) | [x] |
| 10 | `half2float` | Negative NaN: sign 1, exponent 31, fraction `1..0x3ff` | [x] |

Rows 1, 4, 6, and 9 are singleton input classes. All non-singleton rows require
fixed-seed randomized coverage, and the complete 65,536-value domain is also
checked exhaustively.
