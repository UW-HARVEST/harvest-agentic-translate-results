# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table and CMake declares no options or
conditional source selection. There is exactly one valid feature combination:

| # | Cargo feature combination | C configuration |
|---|---------------------------|-----------------|
| 1 | `--no-default-features` (empty feature set) | default CMake configuration |

## Runtime configurations

The rows below come from all five C-defined dynamic exports, including the
low-level functions not declared in the public header. Random cases for scalar
and string rows include boundary values within the branch named by the row.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | `line != NULL`; randomized NUL-terminated byte strings, including empty and long strings | [x] |
| 2 | `printLine` | `line == NULL`; explicit null-special-case branch | [x] |
| 3 | `printIntLine` | randomized `int` values, including `INT_MIN`, `-1`, `0`, `1`, and `INT_MAX` | [x] |
| 4 | `bad` | direct low-level call; fixed 10-byte `alloca`, ten-element zero source, copy count 10 | [x] |
| 5 | `good` | direct low-level call; fixed `10*sizeof(int)` allocation, ten-element zero source, copy count 10 | [x] |
| 6 | `driver`, `bad` | `useGood == 0`; dispatch to `bad` | [x] |
| 7 | `driver`, `good` | `useGood != 0`; randomized positive and negative `int`, dispatch to `good` | [x] |
