# Configuration-Surface Table

Mechanically derived from all five public dynamic symbols, the null/range
branches in `driver.c`, the two composed call paths, and the distinct scalar
shapes consumed by C formatting, floating division, `fabs`, and float-to-int
conversion. There are no compile-time Cargo features or C preprocessor feature
flags.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| C1 | `printLine` | Non-null C string: empty string. | [x] |
| C2 | `printLine` | Non-null C string: randomized non-empty printable bytes, excluding interior NUL. | [x] |
| C3 | `printIntLine` | Randomized negative `int`, including `INT_MIN`. | [x] |
| C4 | `printIntLine` | Zero. | [x] |
| C5 | `printIntLine` | Randomized positive `int`, including `INT_MAX`. | [x] |
| C6 | `bad` | Positive finite `data` whose `100.0 / data` converts to an in-range positive `int`; include exact and fractional quotients. | [x] |
| C7 | `bad` | Negative finite `data` whose quotient converts to an in-range negative `int`; include exact and fractional quotients. | [x] |
| C8 | `bad` | Positive or negative finite nonzero `data` whose quotient is outside the C `int` range, including subnormals and threshold-adjacent magnitudes. | [x] |
| C9 | `bad` | `data` is `+0.0` or `-0.0`. | [x] |
| C10 | `bad` | `data` is `+infinity`, `-infinity`, or NaN. | [x] |
| C11 | `good` | Positive finite `data > 0.000001`; fixed `goodG2B` output followed by an in-range positive quotient. | [x] |
| C12 | `good` | Negative finite `data < -0.000001`; fixed output followed by an in-range negative quotient. | [x] |
| C13 | `good` | `fabs(data) > 0.000001` immediately beyond the threshold; quotient is the largest range (near 100,000,000) reachable through the accepted branch. | [x] |
| C14 | `good` | `data` is `+infinity` or `-infinity`; accepted `fabs` branch and zero quotient. | [x] |
| C15 | `driver` | Accepted positive `goodData`; positive in-range `badData`. | [x] |
| C16 | `driver` | Accepted negative `goodData`; negative in-range `badData`. | [x] |
| C17 | `driver` | Accepted threshold-adjacent `goodData`; out-of-range-conversion `badData`. | [x] |
| C18 | `driver` | Accepted infinite `goodData`; infinite or NaN `badData`. | [x] |
| C19 | `driver` | Rejected near-zero `goodData`; nonzero in-range `badData` (composed pipeline continues after warning). | [x] |
| C20 | `driver` | Rejected NaN `goodData`; zero-sign `badData` (composed pipeline continues after warning). | [x] |

## Feature combinations

`Cargo.toml` declares no `[features]` table. The sole build configuration is
the default/no-feature build, and `--no-default-features` is equivalent.
