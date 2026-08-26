# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` section, and `c_src/CMakeLists.txt` has no
configuration options or conditional source selection. There is exactly one
valid feature set:

| # | Cargo feature set | Cargo invocation | C configuration | verified |
|---|-------------------|------------------|-----------------|----------|
| 1 | empty set | `cargo test --no-default-features` | default CMake configuration | [x] |

## Runtime Configurations

The API has no runtime options, flags, state, element types, formats, byte
order, or aggregate input shapes. Its two scalar `int` inputs branch on sign,
`INT_MIN`, exact divisibility, and whether the computed remainder is negative.
Randomized coverage for each row includes applicable values adjacent to zero
and `INT_MIN`/`INT_MAX`.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `div_euclid` | `v1 >= 0`, `v2 > 0`; direct nonnegative division | [x] |
| 2 | `div_euclid` | `v1 >= 0`, `INT_MIN < v2 < 0`; normalized negative divisor | [x] |
| 3 | `div_euclid` | `v1 >= 0`, `v2 == INT_MIN` | [x] |
| 4 | `div_euclid` | `INT_MIN < v1 < 0`, `v2 > 0`, exactly divisible (`r == 0`) | [x] |
| 5 | `div_euclid` | `INT_MIN < v1 < 0`, `v2 > 0`, nonzero remainder (`r < 0`) | [x] |
| 6 | `div_euclid` | `INT_MIN < v1 < 0`, `INT_MIN < v2 < 0`, exactly divisible (`r == 0`) | [x] |
| 7 | `div_euclid` | `INT_MIN < v1 < 0`, `INT_MIN < v2 < 0`, nonzero remainder (`r < 0`) | [x] |
| 8 | `div_euclid` | `INT_MIN < v1 < 0`, `v2 == INT_MIN` | [x] |
| 9 | `div_euclid` | `v1 == INT_MIN`, `v2 > 0`, exactly divisible (`r == 0`) | [x] |
| 10 | `div_euclid` | `v1 == INT_MIN`, `v2 > 0`, nonzero remainder (`r < 0`) | [x] |
| 11 | `div_euclid` | `v1 == INT_MIN`, `INT_MIN < v2 < 0`, exactly divisible (`r == 0`) | [x] |
| 12 | `div_euclid` | `v1 == INT_MIN`, `INT_MIN < v2 < 0`, nonzero remainder (`r < 0`) | [x] |
| 13 | `div_euclid` | `v1 == INT_MIN`, `v2 == INT_MIN` | [x] |
