# Configuration Surface

## Build-Time Matrix

`Cargo.toml` has no `[features]` table, default features, or optional
dependencies. `c_src/CMakeLists.txt` has no options, conditional compilation,
or conditional source files. Therefore the full valid build-time matrix has
one member:

| # | Cargo invocation | CMake configuration | |
|---|------------------|---------------------|---|
| B01 | `cargo ... --no-default-features` (no feature names) | default configuration with `CMAKE_POSITION_INDEPENDENT_CODE=ON` | [x] |

## Runtime Matrix

Rows come from all 10 C dynamic entry points and every branch-dependent valid
shape in `c_src/src/lib.c`. Randomized rows include fixed edge values as well
as fixed-seed generated values. The native C `int` representation and host byte
order are used through the FFI boundary.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|---|
| C01 | `multiply_with_static` | random and boundary pairs of C `int` values | [x] |
| C02 | `add_with_static` | random and boundary pairs of C `int` values | [x] |
| C03 | `xor_operation` | random and boundary pairs of C `int` values | [x] |
| C04 | `shift_with_static` | positive, zero, negative, and boundary C `int` pairs; fixed shift amount `2` | [x] |
| C05 | `get_operation`, `multiply_with_static` | first lazy-table lookup with opcode `0`, then invoke returned pointer | [x] |
| C06 | `get_operation`, `multiply_with_static` | initialized-table lookup with opcode `0`, then randomized invocation | [x] |
| C07 | `get_operation`, `add_with_static` | initialized-table lookup with opcode `1`, then randomized invocation | [x] |
| C08 | `get_operation`, `xor_operation` | initialized-table lookup with opcode `2`, then randomized invocation | [x] |
| C09 | `get_operation`, `shift_with_static` | initialized-table lookup with opcode `3`, then randomized invocation | [x] |
| C10 | `execute_operation`, `multiply_with_static` | non-null multiply callback and operation name, randomized arguments | [x] |
| C11 | `execute_operation`, `add_with_static` | non-null add callback and operation name, randomized arguments | [x] |
| C12 | `execute_operation`, `xor_operation` | non-null XOR callback and operation name, randomized arguments | [x] |
| C13 | `execute_operation`, `shift_with_static` | non-null shift callback and operation name, randomized arguments | [x] |
| C14 | `compute_checksum` | non-null array, `count == 1` | [x] |
| C15 | `compute_checksum` | non-null array, `count == 2` | [x] |
| C16 | `compute_checksum` | non-null array, `count == 3` | [x] |
| C17 | `compute_checksum` | non-null array, `count == 4` boundary | [x] |
| C18 | `compute_checksum` | non-null array, `count > 4` (including `INT_MAX`); only first four integers copied | [x] |
| C19 | `init_state` | non-null state and random/boundary initial accumulator; all 12 output bytes compared | [x] |
| C20 | `apply_operation`, `multiply_with_static` | non-null arbitrary state and multiply callback | [x] |
| C21 | `apply_operation`, `add_with_static` | non-null arbitrary state and add callback | [x] |
| C22 | `apply_operation`, `xor_operation` | non-null arbitrary state and XOR callback | [x] |
| C23 | `apply_operation`, `shift_with_static` | non-null arbitrary state and shift callback | [x] |
| C24 | `checkshift` | complete composed pipeline with random and boundary four-integer inputs | [x] |

