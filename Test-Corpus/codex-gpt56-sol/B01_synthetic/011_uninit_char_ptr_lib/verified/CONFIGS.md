# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, implicit features, or default features.
`c_src/CMakeLists.txt` has no options, conditional sources, or compile-time
configuration branches. There is exactly one valid feature combination:

| # | Cargo invocation | CMake configuration |
|---|------------------|---------------------|
| 1 | `--no-default-features` (empty feature set) | default |

## Runtime Configurations

Mechanically derived from all four C exports, the nullness branch in
`printLine`, and the zero/nonzero branch in `driver`. Valid C strings include
empty, one-byte, and many-byte shapes; randomized cases cover the full class.
For the integer branch, randomized positive and negative values include
`INT_MIN` and `INT_MAX`.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|-------------------------------------------|--------|
| 1 | `printLine` | non-null valid C string: empty, one byte, and many bytes | [x] |
| 2 | `printLine` | null pointer; accepted as a no-output operation | [x] |
| 3 | `good` | no arguments; fixed `"string"` data path | [x] |
| 4 | `bad` | no arguments; local pointer is uninitialized and passed to `printLine` | [x] |
| 5 | `driver` | `useGood == 0`; dispatch to `bad` | [x] |
| 6 | `driver` | `useGood != 0`; randomized positive/negative values dispatch to `good` | [x] |
