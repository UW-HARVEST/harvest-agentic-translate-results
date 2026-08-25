# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` declares no
options or preprocessor definitions. There is exactly one valid combination:

| # | Cargo features | CMake options | check |
|---|----------------|---------------|-------|
| 1 | none (`--no-default-features`) | defaults | [x] |

## Runtime Configurations

The only public entry point is `memchra2`. The rows below are the cross-product
of the branches driven by public inputs:

- `a` has seven classes under its bit reinterpretation as `float`: positive
  zero, `(0, 1)`, `[1, 1000)`, finite `[1000, +inf)`, positive infinity,
  positive NaN, and sign-bit-set values.
- Each of `b`, `c`, and `d` is independently nonnegative (`+`) or negative
  (`-`). This changes the formatted hyphen count.

Every row also randomizes all four integer values and deliberately includes
low-byte values `00`, `7f`, `80`, and `ff`, decimal-width boundaries,
`INT_MIN`/`INT_MAX`, wrapping sums, native-endian byte interpretation, and
XOR low-byte behavior. These axes do not create C control-flow branches, so
they are corpus requirements rather than redundant table dimensions.

| # | entry point(s) | configuration (options set + input shape) | check |
|---|----------------|--------------------------------------------|-------|
| 1 | `memchra2` | `a`: `+0.0` bits; `b/c/d`: `+/+/+` | [x] |
| 2 | `memchra2` | `a`: `+0.0` bits; `b/c/d`: `+/+/-` | [x] |
| 3 | `memchra2` | `a`: `+0.0` bits; `b/c/d`: `+/-/+` | [x] |
| 4 | `memchra2` | `a`: `+0.0` bits; `b/c/d`: `+/-/-` | [x] |
| 5 | `memchra2` | `a`: `+0.0` bits; `b/c/d`: `-/+/+` | [x] |
| 6 | `memchra2` | `a`: `+0.0` bits; `b/c/d`: `-/+/-` | [x] |
| 7 | `memchra2` | `a`: `+0.0` bits; `b/c/d`: `-/-/+` | [x] |
| 8 | `memchra2` | `a`: `+0.0` bits; `b/c/d`: `-/-/-` | [x] |
| 9 | `memchra2` | `a` float bits: `(0, 1)`; `b/c/d`: `+/+/+` | [x] |
| 10 | `memchra2` | `a` float bits: `(0, 1)`; `b/c/d`: `+/+/-` | [x] |
| 11 | `memchra2` | `a` float bits: `(0, 1)`; `b/c/d`: `+/-/+` | [x] |
| 12 | `memchra2` | `a` float bits: `(0, 1)`; `b/c/d`: `+/-/-` | [x] |
| 13 | `memchra2` | `a` float bits: `(0, 1)`; `b/c/d`: `-/+/+` | [x] |
| 14 | `memchra2` | `a` float bits: `(0, 1)`; `b/c/d`: `-/+/-` | [x] |
| 15 | `memchra2` | `a` float bits: `(0, 1)`; `b/c/d`: `-/-/+` | [x] |
| 16 | `memchra2` | `a` float bits: `(0, 1)`; `b/c/d`: `-/-/-` | [x] |
| 17 | `memchra2` | `a` float bits: `[1, 1000)`; `b/c/d`: `+/+/+` | [x] |
| 18 | `memchra2` | `a` float bits: `[1, 1000)`; `b/c/d`: `+/+/-` | [x] |
| 19 | `memchra2` | `a` float bits: `[1, 1000)`; `b/c/d`: `+/-/+` | [x] |
| 20 | `memchra2` | `a` float bits: `[1, 1000)`; `b/c/d`: `+/-/-` | [x] |
| 21 | `memchra2` | `a` float bits: `[1, 1000)`; `b/c/d`: `-/+/+` | [x] |
| 22 | `memchra2` | `a` float bits: `[1, 1000)`; `b/c/d`: `-/+/-` | [x] |
| 23 | `memchra2` | `a` float bits: `[1, 1000)`; `b/c/d`: `-/-/+` | [x] |
| 24 | `memchra2` | `a` float bits: `[1, 1000)`; `b/c/d`: `-/-/-` | [x] |
| 25 | `memchra2` | `a` float bits: finite `[1000, +inf)`; `b/c/d`: `+/+/+` | [x] |
| 26 | `memchra2` | `a` float bits: finite `[1000, +inf)`; `b/c/d`: `+/+/-` | [x] |
| 27 | `memchra2` | `a` float bits: finite `[1000, +inf)`; `b/c/d`: `+/-/+` | [x] |
| 28 | `memchra2` | `a` float bits: finite `[1000, +inf)`; `b/c/d`: `+/-/-` | [x] |
| 29 | `memchra2` | `a` float bits: finite `[1000, +inf)`; `b/c/d`: `-/+/+` | [x] |
| 30 | `memchra2` | `a` float bits: finite `[1000, +inf)`; `b/c/d`: `-/+/-` | [x] |
| 31 | `memchra2` | `a` float bits: finite `[1000, +inf)`; `b/c/d`: `-/-/+` | [x] |
| 32 | `memchra2` | `a` float bits: finite `[1000, +inf)`; `b/c/d`: `-/-/-` | [x] |
| 33 | `memchra2` | `a`: positive infinity bits; `b/c/d`: `+/+/+` | [x] |
| 34 | `memchra2` | `a`: positive infinity bits; `b/c/d`: `+/+/-` | [x] |
| 35 | `memchra2` | `a`: positive infinity bits; `b/c/d`: `+/-/+` | [x] |
| 36 | `memchra2` | `a`: positive infinity bits; `b/c/d`: `+/-/-` | [x] |
| 37 | `memchra2` | `a`: positive infinity bits; `b/c/d`: `-/+/+` | [x] |
| 38 | `memchra2` | `a`: positive infinity bits; `b/c/d`: `-/+/-` | [x] |
| 39 | `memchra2` | `a`: positive infinity bits; `b/c/d`: `-/-/+` | [x] |
| 40 | `memchra2` | `a`: positive infinity bits; `b/c/d`: `-/-/-` | [x] |
| 41 | `memchra2` | `a`: positive NaN bits; `b/c/d`: `+/+/+` | [x] |
| 42 | `memchra2` | `a`: positive NaN bits; `b/c/d`: `+/+/-` | [x] |
| 43 | `memchra2` | `a`: positive NaN bits; `b/c/d`: `+/-/+` | [x] |
| 44 | `memchra2` | `a`: positive NaN bits; `b/c/d`: `+/-/-` | [x] |
| 45 | `memchra2` | `a`: positive NaN bits; `b/c/d`: `-/+/+` | [x] |
| 46 | `memchra2` | `a`: positive NaN bits; `b/c/d`: `-/+/-` | [x] |
| 47 | `memchra2` | `a`: positive NaN bits; `b/c/d`: `-/-/+` | [x] |
| 48 | `memchra2` | `a`: positive NaN bits; `b/c/d`: `-/-/-` | [x] |
| 49 | `memchra2` | `a`: sign bit set; `b/c/d`: `+/+/+` | [x] |
| 50 | `memchra2` | `a`: sign bit set; `b/c/d`: `+/+/-` | [x] |
| 51 | `memchra2` | `a`: sign bit set; `b/c/d`: `+/-/+` | [x] |
| 52 | `memchra2` | `a`: sign bit set; `b/c/d`: `+/-/-` | [x] |
| 53 | `memchra2` | `a`: sign bit set; `b/c/d`: `-/+/+` | [x] |
| 54 | `memchra2` | `a`: sign bit set; `b/c/d`: `-/+/-` | [x] |
| 55 | `memchra2` | `a`: sign bit set; `b/c/d`: `-/-/+` | [x] |
| 56 | `memchra2` | `a`: sign bit set; `b/c/d`: `-/-/-` | [x] |

