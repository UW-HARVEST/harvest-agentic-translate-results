# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no CMake
options or conditional definitions. There is exactly one valid feature
combination:

| # | Cargo feature combination | C configuration | Status |
|---|---------------------------|-----------------|--------|
| 1 | `<none>` (`--no-default-features --features ''`) | Unconditional `driver` source | [x] `cargo check` |

## Runtime and Input Configurations

Mechanical search of the complete C source found no mode, option, flag,
`switch`, element type, format selector, byte-order selector, count, width, or
length axis. `printLine` has the sole runtime branch (`line != NULL`); its null
side is row 1 of `ERRORS.md`. The table includes every `nm`-visible entry point,
starting with the low-level functions and ending with the composed `main`.

| # | entry point(s) | configuration (options set + input shape) | Status |
|---|----------------|-------------------------------------------|--------|
| 1 | `printLine` | Non-null NUL-terminated byte string; randomized empty and non-empty strings | [x] |
| 2 | `printIntLine` | Any C `int`; randomized values including `INT_MIN`, `-1`, `0`, `1`, and `INT_MAX` | [x] |
| 3 | `bad` | No inputs; executes the unchanged-expression path and prints two integer lines | [x] |
| 4 | `good` | No inputs; executes the assigned-sum path and prints two integer lines | [x] |
| 5 | `main` | Composed end-to-end call; randomized ignored `argc` with valid or null `argv` | [x] |
