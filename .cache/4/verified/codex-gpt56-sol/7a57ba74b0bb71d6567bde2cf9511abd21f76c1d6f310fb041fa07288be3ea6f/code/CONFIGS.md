# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` declares no
options or conditional sources. Therefore the complete build-time
configuration set has one member:

| # | Cargo invocation | CMake configuration |
|---|------------------|---------------------|
| B1 | `cargo test --no-default-features` | Default configuration with `-DCMAKE_POSITION_INDEPENDENT_CODE=ON` |

## Runtime Configurations

The entry points come from `nm -D --defined-only`, including `printLine`, which
is exported even though it is not declared in the public header. Rows are
derived from the null check, the `data < 100` branch, the fixed 100-byte arrays,
the 99-byte source payload, and the C-string output operation.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `printLine` | Non-null pointer to an empty C string. | [x] |
| C2 | `printLine` | Non-null pointer to a one-byte C string. | [x] |
| C3 | `printLine` | Non-null pointer to a many-byte C string; randomized bytes contain no interior NUL. | [x] |
| C4 | `driver` -> `printLine` | `data == 0`; copied prefix is empty. | [x] |
| C5 | `driver` -> `printLine` | `data == 1`; copied prefix has one `A`. | [x] |
| C6 | `driver` -> `printLine` | `2 <= data <= 98`; randomized interior prefix lengths. | [x] |
| C7 | `driver` -> `printLine` | `data == 99`; maximum source payload is copied. | [x] |
| C8 | `driver` -> `printLine` | `data >= 100`; randomized values including 100 and `INT_MAX`, copy branch is skipped. | [x] |

There are no runtime modes, option setters, flags, element types, formats, byte
orders, counts, or enums. `driver(data < 0)` is omitted because its C behavior
is undefined, as recorded in `ERRORS.md`.
