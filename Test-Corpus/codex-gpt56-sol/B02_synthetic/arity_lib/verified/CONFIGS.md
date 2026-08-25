# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and CMake has no options or conditional
definitions. There is one valid build-time configuration:

| # | Cargo invocation | CMake configuration |
|---|------------------|---------------------|
| 1 | `--no-default-features` with no named features | default |

## Runtime Configurations

Rows come from every `if`, `else if`, `switch`, loop-bound shape, and composed
public entry point in `c_src/src/lib.c`. Randomized arithmetic inputs are kept
within ranges where C signed arithmetic is defined.

For the `arity4`, `arity2`, and `arity3` rows, `param1` classes are:

- Z: zero (mask operation 0 and non-positive allocation value)
- P0: positive and divisible by 4
- P1, P2, P3: positive with remainder 1, 2, or 3
- N0: negative and divisible by 4
- ND: negative with remainder -1, -2, or -3 (default mask branch)

`param3` and `param4` use Z for zero and NZ for randomized nonzero values.
`param2` is randomized in every composed-operation row.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `shift_array` | Empty array, `size = 0`, `positions = 0`; no-op | [x] |
| 2 | `shift_array` | Nonempty array, `positions < 0`; no-op | [x] |
| 3 | `shift_array` | Nonempty array, `positions = 0`; no-op | [x] |
| 4 | `shift_array` | Nonempty array, `positions >= size`; no-op | [x] |
| 5 | `shift_array` | Minimum active shape, `size = 2`, `positions = 1` | [x] |
| 6 | `shift_array` | Many elements, interior `1 <= positions < size` including `size - 1` | [x] |
| 7 | `process_string` | Empty C string | [x] |
| 8 | `process_string` | One-byte C string | [x] |
| 9 | `process_string` | Multi-byte C string with randomized non-NUL bytes | [x] |
| 10 | `apply_bitmask` | Operation 0 (`value & 0xf0`), randomized signed values | [x] |
| 11 | `apply_bitmask` | Operation 1 (`value & 0x0f`), randomized signed values | [x] |
| 12 | `apply_bitmask` | Operation 2 (`value \| 0xaa`), randomized signed values | [x] |
| 13 | `apply_bitmask` | Operation 3 (`value ^ 0x55`), randomized signed values | [x] |
| 14 | `apply_bitmask` | Operation below 0, randomized signed values | [x] |
| 15 | `apply_bitmask` | Operation above 3, randomized signed values | [x] |
| 16 | `init_matrix` | One writable contiguous `3 x 4` integer matrix | [x] |
| 17 | `compare_allocations` | `val1 <= 0`, randomized `val1` and `val2` | [x] |
| 18 | `compare_allocations` | `val1 > 0`, randomized `val1` and `val2` | [x] |
| 19 | `arity4` | param1 Z, param3 Z, param4 Z | [x] |
| 20 | `arity4` | param1 Z, param3 Z, param4 NZ | [x] |
| 21 | `arity4` | param1 Z, param3 NZ, param4 Z | [x] |
| 22 | `arity4` | param1 Z, param3 NZ, param4 NZ | [x] |
| 23 | `arity4` | param1 P0, param3 Z, param4 Z | [x] |
| 24 | `arity4` | param1 P0, param3 Z, param4 NZ | [x] |
| 25 | `arity4` | param1 P0, param3 NZ, param4 Z | [x] |
| 26 | `arity4` | param1 P0, param3 NZ, param4 NZ | [x] |
| 27 | `arity4` | param1 P1, param3 Z, param4 Z | [x] |
| 28 | `arity4` | param1 P1, param3 Z, param4 NZ | [x] |
| 29 | `arity4` | param1 P1, param3 NZ, param4 Z | [x] |
| 30 | `arity4` | param1 P1, param3 NZ, param4 NZ | [x] |
| 31 | `arity4` | param1 P2, param3 Z, param4 Z | [x] |
| 32 | `arity4` | param1 P2, param3 Z, param4 NZ | [x] |
| 33 | `arity4` | param1 P2, param3 NZ, param4 Z | [x] |
| 34 | `arity4` | param1 P2, param3 NZ, param4 NZ | [x] |
| 35 | `arity4` | param1 P3, param3 Z, param4 Z | [x] |
| 36 | `arity4` | param1 P3, param3 Z, param4 NZ | [x] |
| 37 | `arity4` | param1 P3, param3 NZ, param4 Z | [x] |
| 38 | `arity4` | param1 P3, param3 NZ, param4 NZ | [x] |
| 39 | `arity4` | param1 N0, param3 Z, param4 Z | [x] |
| 40 | `arity4` | param1 N0, param3 Z, param4 NZ | [x] |
| 41 | `arity4` | param1 N0, param3 NZ, param4 Z | [x] |
| 42 | `arity4` | param1 N0, param3 NZ, param4 NZ | [x] |
| 43 | `arity4` | param1 ND, param3 Z, param4 Z | [x] |
| 44 | `arity4` | param1 ND, param3 Z, param4 NZ | [x] |
| 45 | `arity4` | param1 ND, param3 NZ, param4 Z | [x] |
| 46 | `arity4` | param1 ND, param3 NZ, param4 NZ | [x] |
| 47 | `arity2` | param1 Z; implicit param3 Z and param4 Z | [x] |
| 48 | `arity2` | param1 P0; implicit param3 Z and param4 Z | [x] |
| 49 | `arity2` | param1 P1; implicit param3 Z and param4 Z | [x] |
| 50 | `arity2` | param1 P2; implicit param3 Z and param4 Z | [x] |
| 51 | `arity2` | param1 P3; implicit param3 Z and param4 Z | [x] |
| 52 | `arity2` | param1 N0; implicit param3 Z and param4 Z | [x] |
| 53 | `arity2` | param1 ND; implicit param3 Z and param4 Z | [x] |
| 54 | `arity3` | param1 Z, param3 Z; implicit param4 Z | [x] |
| 55 | `arity3` | param1 Z, param3 NZ; implicit param4 Z | [x] |
| 56 | `arity3` | param1 P0, param3 Z; implicit param4 Z | [x] |
| 57 | `arity3` | param1 P0, param3 NZ; implicit param4 Z | [x] |
| 58 | `arity3` | param1 P1, param3 Z; implicit param4 Z | [x] |
| 59 | `arity3` | param1 P1, param3 NZ; implicit param4 Z | [x] |
| 60 | `arity3` | param1 P2, param3 Z; implicit param4 Z | [x] |
| 61 | `arity3` | param1 P2, param3 NZ; implicit param4 Z | [x] |
| 62 | `arity3` | param1 P3, param3 Z; implicit param4 Z | [x] |
| 63 | `arity3` | param1 P3, param3 NZ; implicit param4 Z | [x] |
| 64 | `arity3` | param1 N0, param3 Z; implicit param4 Z | [x] |
| 65 | `arity3` | param1 N0, param3 NZ; implicit param4 Z | [x] |
| 66 | `arity3` | param1 ND, param3 Z; implicit param4 Z | [x] |
| 67 | `arity3` | param1 ND, param3 NZ; implicit param4 Z | [x] |
| 68 | `arity` | Effective length 2; dispatch to `arity2` | [x] |
| 69 | `arity` | Effective length 3; dispatch to `arity3` | [x] |
| 70 | `arity` | Effective length exactly 4; dispatch to `arity4` | [x] |
| 71 | `arity` | Effective length 5 through 255; only the first four values are read | [x] |
| 72 | `arity` | Caller `int len` outside `0..=255`; low byte aliases a valid effective length | [x] |
