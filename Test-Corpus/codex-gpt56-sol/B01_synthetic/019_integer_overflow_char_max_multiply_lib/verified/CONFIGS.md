# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and no optional dependencies.
`c_src/CMakeLists.txt` has no project options or conditional source selection.
There is exactly one behavioral build configuration:

| # | Rust feature combination | C configuration | Check command | |
|---|--------------------------|-----------------|---------------|-|
| 1 | Empty set (`--no-default-features`) | Default CMake configuration with position-independent code | `cargo check --no-default-features` | [x] |

## Runtime Configurations

These rows cover every C-defined dynamic entry point, including functions not
declared in the public header. Singleton rows are exhaustive; ranged rows use
fixed-seed randomized values.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|-|
| 1 | `printLine` | Non-null empty C string | [x] |
| 2 | `printLine` | Non-null one-byte C string, randomized non-NUL byte | [x] |
| 3 | `printLine` | Non-null multi-byte C string, randomized non-NUL bytes and lengths | [x] |
| 4 | `printHexCharLine` | Negative signed `char`, randomized across `CHAR_MIN..=-1` | [x] |
| 5 | `printHexCharLine` | Zero signed `char` boundary | [x] |
| 6 | `printHexCharLine` | Positive signed `char`, randomized across `1..=CHAR_MAX` | [x] |
| 7 | `bad` | No input; internal `data = CHAR_MAX` and unchecked wrapping multiplication | [x] |
| 8 | `good` | No input; sequential `goodG2B` then `goodB2G` pipeline | [x] |
| 9 | `driver` | `useGood == 0`, selecting `bad` | [x] |
| 10 | `driver` | Randomized nonzero `int`, positive and negative, selecting `good` | [x] |

The only remaining public input shape is `printLine(NULL)`, which is a
rejection and is tracked in `ERRORS.md`.
