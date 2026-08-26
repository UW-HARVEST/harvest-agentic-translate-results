# Configuration Surface

## Build-time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` defines no
options or conditional sources. The only valid build-time combination is:

| # | Rust feature combination | C configuration | status |
|---|--------------------------|-----------------|--------|
| 1 | empty set (`--no-default-features --features ''`) | default CMake configuration | [x] compiles |

## Runtime Configurations

The rows below are the cross-product pruned to branches and input shapes that
`dataentry` actually distinguishes. Every row randomizes all unconstrained
parameters with a fixed seed. Mode 3 has one row per selected lookup-table
cell because the C data differs for all 12 cells.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `dataentry` -> `create_entries`, `find_entry` | mode 1; default count (`param1 <= 0`, count 5); target at first/middle/last valid index | [x] |
| 2 | `dataentry` -> `create_entries`, `find_entry` | mode 1; default count; target below first index (`param2 < 0`) | [x] |
| 3 | `dataentry` -> `create_entries`, `find_entry` | mode 1; default count; target at/above end (`param2 >= 5`) | [x] |
| 4 | `dataentry` -> `create_entries`, `find_entry` | mode 1; explicit one entry (`param1 == 1`); target index 0 | [x] |
| 5 | `dataentry` -> `create_entries`, `find_entry` | mode 1; explicit one entry; target below first index | [x] |
| 6 | `dataentry` -> `create_entries`, `find_entry` | mode 1; explicit one entry; target at/above end | [x] |
| 7 | `dataentry` -> `create_entries`, `find_entry` | mode 1; explicit many entries (`param1 > 1`); target at first/middle/last valid index | [x] |
| 8 | `dataentry` -> `create_entries`, `find_entry` | mode 1; explicit many entries; target below first index | [x] |
| 9 | `dataentry` -> `create_entries`, `find_entry` | mode 1; explicit many entries; target at/above end | [x] |
| 10 | `dataentry` -> `create_entries`, `modify_entries` | mode 2; default count (`param1 <= 0`, count 3); zero multiplier, so total is zero and `param3` is not added | [x] |
| 11 | `dataentry` -> `create_entries`, `modify_entries` | mode 2; default count; nonzero multiplier, so nonzero total receives `param3` | [x] |
| 12 | `dataentry` -> `create_entries`, `modify_entries` | mode 2; explicit one entry (`param1 == 1`); zero multiplier | [x] |
| 13 | `dataentry` -> `create_entries`, `modify_entries` | mode 2; explicit one entry; nonzero multiplier | [x] |
| 14 | `dataentry` -> `create_entries`, `modify_entries` | mode 2; explicit many entries (`param1 > 1`); zero multiplier | [x] |
| 15 | `dataentry` -> `create_entries`, `modify_entries` | mode 2; explicit many entries; nonzero multiplier | [x] |
| 16 | `dataentry` -> `calculate_lookup` | mode 3; row 0, column 0 | [x] |
| 17 | `dataentry` -> `calculate_lookup` | mode 3; row 0, column 1 | [x] |
| 18 | `dataentry` -> `calculate_lookup` | mode 3; row 0, column 2 | [x] |
| 19 | `dataentry` -> `calculate_lookup` | mode 3; row 1, column 0 | [x] |
| 20 | `dataentry` -> `calculate_lookup` | mode 3; row 1, column 1 | [x] |
| 21 | `dataentry` -> `calculate_lookup` | mode 3; row 1, column 2 | [x] |
| 22 | `dataentry` -> `calculate_lookup` | mode 3; row 2, column 0 | [x] |
| 23 | `dataentry` -> `calculate_lookup` | mode 3; row 2, column 1 | [x] |
| 24 | `dataentry` -> `calculate_lookup` | mode 3; row 2, column 2 | [x] |
| 25 | `dataentry` -> `calculate_lookup` | mode 3; row 3, column 0 | [x] |
| 26 | `dataentry` -> `calculate_lookup` | mode 3; row 3, column 1 | [x] |
| 27 | `dataentry` -> `calculate_lookup` | mode 3; row 3, column 2 | [x] |
| 28 | `dataentry` -> `process_name` | any default-switch mode (`mode < 1 || mode > 3`); negative/zero/positive/extreme `param1`; other parameters ignored | [x] |
