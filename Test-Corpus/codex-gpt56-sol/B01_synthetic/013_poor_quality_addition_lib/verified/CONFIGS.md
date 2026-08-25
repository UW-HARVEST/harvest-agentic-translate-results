# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no
configuration options or conditional sources. The complete valid feature
combination set is:

| # | Cargo invocation | C configuration |
|---|------------------|-----------------|
| 1 | `--no-default-features` (empty feature set) | CMake defaults with position-independent code |

## Runtime Configurations

The C API exposes no runtime options, modes, flags, lengths, formats, byte
orders, element types, or enums. Rows below enumerate every exported entry
point and every input shape on which the C source takes a distinct valid path.
The null `printLine` path is tracked in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `printLine` | Non-null, NUL-terminated byte string; randomized empty and non-empty contents | [x] |
| 2 | `printIntLine` | Any C `int`; randomized across the full `int32_t` domain including boundaries | [x] |
| 3 | `bad` | No options or inputs; direct low-level call | [x] |
| 4 | `good` | No options or inputs; direct low-level call | [x] |
| 5 | `driver` | No options or inputs; complete composed call sequence | [x] |
