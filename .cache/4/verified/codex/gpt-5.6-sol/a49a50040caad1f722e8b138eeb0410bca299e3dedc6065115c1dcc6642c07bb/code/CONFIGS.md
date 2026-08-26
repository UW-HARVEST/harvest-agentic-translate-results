# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and CMake defines no options or
configuration macros. There is exactly one valid feature combination:

| # | Cargo invocation | CMake configuration | status |
|---|------------------|---------------------|-----|
| 1 | `--no-default-features` (empty feature set) | Default `CMakeLists.txt` configuration | [x] |

## Runtime Configurations

These rows cover every defined dynamic symbol. They split the two actual C
branches (`line != NULL` and `useGood != 0`) and the C-string shapes consumed
by `%s`. Randomized rows use a fixed seed and include boundary sizes.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|-------------------------------------------|-----|
| 1 | `printLine` | Non-null empty C string; randomized ignored bytes after the first NUL | [x] |
| 2 | `printLine` | Non-null C string with exactly one visible non-NUL byte | [x] |
| 3 | `printLine` | Non-null C string with 2..4096 visible non-NUL bytes, including non-UTF-8 bytes | [x] |
| 4 | `printLine` | Non-null buffer with an interior NUL and randomized non-NUL prefix/suffix | [x] |
| 5 | `bad` | No arguments; execute the automatic-array helper path | [x] |
| 6 | `good` | No arguments; execute the static-array helper path | [x] |
| 7 | `driver` | `useGood == 0`, selecting `bad` | [x] |
| 8 | `driver` | `useGood != 0`, selecting `good`; randomized positive/negative values including `INT_MIN` and `INT_MAX` | [x] |

The static `helperBad` and `helperGood1` functions are not ELF exports and
cannot be called through the external FFI boundary; rows 5 through 8 exercise
them through every exported caller.
