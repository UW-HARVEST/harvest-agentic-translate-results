# Configuration Surface

The public surface is the union of `driver.h` and all functions exported by
the C shared object. There are no runtime options, modes, flags, element
types, lengths, byte-order settings, preprocessor feature branches, or Cargo
features. The meaningful data-shape matrix comes from the null branch in
`printLine`, the floating comparison in `goodB2G`, C integer formatting, and
the division/conversion operations.

| # | entry point(s) | configuration (options set + input shape) | Status |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | Non-null pointer to an empty C string. | [x] |
| 2 | `printLine` | Non-null pointer to a non-empty, NUL-terminated byte string. | [x] |
| 3 | `printIntLine` | Negative integer values. | [x] |
| 4 | `printIntLine` | Zero. | [x] |
| 5 | `printIntLine` | Positive integer values. | [x] |
| 6 | `printIntLine` | Exact `INT_MIN` and `INT_MAX` boundaries. | [x] |
| 7 | `bad` | Positive finite nonzero float with an in-range quotient. | [x] |
| 8 | `bad` | Negative finite nonzero float with an in-range quotient. | [x] |
| 9 | `bad` | Positive and negative zero. | [x] |
| 10 | `bad` | Finite nonzero float whose quotient is outside `int` range. | [x] |
| 11 | `bad` | Positive infinity, negative infinity, and NaN. | [x] |
| 12 | `good` | Positive finite `data` with `fabs(data) > 0.000001`. | [x] |
| 13 | `good` | Negative finite `data` with `fabs(data) > 0.000001`. | [x] |
| 14 | `good` | Positive/negative zero, sub-threshold finite values, and exact positive/negative threshold. | [x] |
| 15 | `good` | NaN, for which the ordered comparison is false. | [x] |
| 16 | `good` | Positive and negative infinity, for which the comparison is true. | [x] |
| 17 | `driver` | Accepted finite positive `goodData`; in-range positive/negative finite `badData`. | [x] |
| 18 | `driver` | Accepted finite negative `goodData`; in-range positive/negative finite `badData`. | [x] |
| 19 | `driver` | Rejected finite threshold-or-smaller `goodData`; in-range finite `badData`. | [x] |
| 20 | `driver` | NaN `goodData`; in-range finite `badData`. | [x] |
| 21 | `driver` | Infinite `goodData`; in-range finite `badData`. | [x] |
| 22 | `driver` | Accepted finite `goodData`; zero, quotient-overflow, infinite, or NaN `badData`. | [x] |
| 23 | `driver` | Rejected finite or NaN `goodData`; zero, quotient-overflow, infinite, or NaN `badData`. | [x] |

## Build Configurations

Cargo.toml declares no features. The complete build-configuration set is:

| # | Cargo arguments | Status |
|---|-----------------|-----|
| 1 | default (`cargo test`) | [x] |
| 2 | no defaults (`cargo test --no-default-features`) | [x] |
