# Configuration Surface

## Build-Time Configurations

Neither `Cargo.toml` nor `c_src/CMakeLists.txt` defines a feature, option, or
backend. There is exactly one valid feature combination:

| # | Cargo features | CMake options |
|---|----------------|---------------|
| 1 | Empty set (`--no-default-features`) | Default configuration |

## Runtime Configurations

Rows are derived from every exported entry point, the `line != NULL` and
`fabs(data) > 0.000001` branches, and the distinct floating-point shapes
consumed by division and C integer conversion. Rejected branch inputs are
tracked in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | Non-null, NUL-terminated input; empty, ordinary, embedded-first-NUL, and long string shapes | [x] |
| 2 | `printIntLine` | `int` zero, positive, negative, `INT_MIN`, `INT_MAX`, and randomized values | [x] |
| 3 | `bad` | Nonzero finite `float` where `100.0 / data` is representable as `int`; positive, negative, boundary, and randomized values | [x] |
| 4 | `bad` | IEEE exceptional/result-boundary shapes: signed zero, subnormal/tiny nonzero, infinities, NaNs, and values immediately around the `int` conversion range | [x] |
| 5 | `good` | Finite `float` with `fabs(data) > 0.000001`; positive, negative, immediately-above-threshold, and randomized values | [x] |
| 6 | `good` | Positive or negative infinity, which takes the division branch and converts signed zero to `int` | [x] |
| 7 | `driver` | `goodData` takes the finite valid division branch; `badData` has a regular representable quotient | [x] |
| 8 | `driver` | `goodData` takes the finite valid division branch; `badData` has an IEEE exceptional or integer-conversion-boundary shape | [x] |
| 9 | `driver` | Infinite `goodData`; regular or exceptional `badData` | [x] |
