# Configuration Surface

The C header has one public entry point and no compile-time or runtime feature
setters. The axes below come directly from the `switch`, fallback ternaries,
loops, lookup bounds, and result-dependent branches in `src/lib.c`.

For count-driven modes, one and many are separate input shapes. Random tests
cover boundary and interior counts within each row. `MAX_ENTRIES` is unused
and therefore does not define a boundary.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `dataentry` mode 1 | `param1 <= 0` selects fallback count 5; target index `param2` is in `0..=4` | [x] |
| 2 | `dataentry` mode 1 | explicit count `param1 == 1`; target index is 0 | [x] |
| 3 | `dataentry` mode 1 | explicit count `param1 > 1`; target index is first, interior, or last | [x] |
| 4 | `dataentry` mode 1 | fallback count 5; target index is below 0 or at/above 5 | [x] |
| 5 | `dataentry` mode 1 | explicit positive count; target index is below 0 or at/above count | [x] |
| 6 | `dataentry` mode 2 | `param1 <= 0` selects fallback count 3; multiplier `param2 == 0`, so total is zero and `param3` is not added | [x] |
| 7 | `dataentry` mode 2 | fallback count 3; nonzero multiplier, so nonzero total takes the `param3` addition branch | [x] |
| 8 | `dataentry` mode 2 | explicit count `param1 == 1`; zero multiplier | [x] |
| 9 | `dataentry` mode 2 | explicit count `param1 == 1`; nonzero multiplier | [x] |
| 10 | `dataentry` mode 2 | explicit count `param1 > 1`; zero multiplier | [x] |
| 11 | `dataentry` mode 2 | explicit count `param1 > 1`; nonzero multiplier | [x] |
| 12 | `dataentry` mode 3 | row in `0..=3` and column in `0..=2`; all table entries are nonzero, so double the lookup value and add `param3` | [x] |
| 13 | `dataentry` mode 3 | row below 0; column arbitrary | [x] |
| 14 | `dataentry` mode 3 | row at/above 4; column arbitrary | [x] |
| 15 | `dataentry` mode 3 | valid row; column below 0 | [x] |
| 16 | `dataentry` mode 3 | valid row; column at/above 3 | [x] |
| 17 | `dataentry` default | mode is any value except 1, 2, or 3; internal name becomes `"TestName"` and result is its length (8) times `param1`; `param2` and `param3` are ignored | [x] |

## Feature Matrix

`Cargo.toml` has no `[features]` table. The only build configuration is the
default feature set.

Verification command:

```sh
timeout 600 cargo test --release --test differential -- --test-threads=1
```
