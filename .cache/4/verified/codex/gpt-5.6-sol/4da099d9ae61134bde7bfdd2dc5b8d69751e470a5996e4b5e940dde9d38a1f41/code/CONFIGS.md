# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no
configuration options or conditional source selection. The complete valid Rust
feature set is therefore:

| # | Cargo invocation feature set | C configuration | status |
|---|------------------------------|-----------------|--------|
| F01 | `--no-default-features` (empty set) | the sole/default CMake build | [x] |

## Runtime Configurations

Rows are derived from the exported definitions, their `if` branches, fixed
loop bounds, and the zero/nonzero tests on each public input. Randomized rows
use a fixed seed and include positive, negative, and wrapping integer values.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| C01 | `init_array`, `free_array` | zero capacity; empty array | [x] |
| C02 | `init_array`, `free_array` | capacity one; empty array | [x] |
| C03 | `init_array`, `free_array` | randomized nonzero capacities greater than one; empty array | [x] |
| C04 | `expand_array` | empty array with capacity one; manual growth | [x] |
| C05 | `expand_array` | populated array with capacity two; manual growth preserves elements | [x] |
| C06 | `add_element` | empty array with spare capacity (`size < capacity`), no growth | [x] |
| C07 | `add_element` | partially populated array with spare capacity (`size < capacity`), no growth | [x] |
| C08 | `add_element`, `expand_array` | exactly full array (`size == capacity`), growth before append | [x] |
| C09 | `init_array`, `add_element`, `expand_array`, `free_array` | many randomized elements causing repeated capacity doubling | [x] |
| C10 | `free_array` | non-null empty and populated arrays | [x] |
| C11 | `process_flags` | low flag nibble `0x0`; arbitrary high bits ignored | [x] |
| C12 | `process_flags` | low flag nibble `0x1`; arbitrary high bits ignored | [x] |
| C13 | `process_flags` | low flag nibble `0x2`; arbitrary high bits ignored | [x] |
| C14 | `process_flags` | low flag nibble `0x3`; arbitrary high bits ignored | [x] |
| C15 | `process_flags` | low flag nibble `0x4`; arbitrary high bits ignored | [x] |
| C16 | `process_flags` | low flag nibble `0x5`; arbitrary high bits ignored | [x] |
| C17 | `process_flags` | low flag nibble `0x6`; arbitrary high bits ignored | [x] |
| C18 | `process_flags` | low flag nibble `0x7`; arbitrary high bits ignored | [x] |
| C19 | `process_flags` | low flag nibble `0x8`; arbitrary high bits ignored | [x] |
| C20 | `process_flags` | low flag nibble `0x9`; arbitrary high bits ignored | [x] |
| C21 | `process_flags` | low flag nibble `0xA`; arbitrary high bits ignored | [x] |
| C22 | `process_flags` | low flag nibble `0xB`; arbitrary high bits ignored | [x] |
| C23 | `process_flags` | low flag nibble `0xC`; arbitrary high bits ignored | [x] |
| C24 | `process_flags` | low flag nibble `0xD`; arbitrary high bits ignored | [x] |
| C25 | `process_flags` | low flag nibble `0xE`; arbitrary high bits ignored | [x] |
| C26 | `process_flags` | low flag nibble `0xF`; arbitrary high bits ignored | [x] |
| C27 | `matrix` | read the default 3-by-4 writable global byte-for-byte | [x] |
| C28 | `matrix` | write randomized 3-by-4 values and read them back byte-for-byte | [x] |
| C29 | `calculate_matrix_checksum`, `matrix` | default matrix values; fixed 3-by-4 traversal | [x] |
| C30 | `calculate_matrix_checksum`, `matrix` | randomized matrix values, including negative and wrapping sums | [x] |
| C31 | `matrixsum` | parameter zero/nonzero mask `0x0` | [x] |
| C32 | `matrixsum` | parameter zero/nonzero mask `0x1` | [x] |
| C33 | `matrixsum` | parameter zero/nonzero mask `0x2` | [x] |
| C34 | `matrixsum` | parameter zero/nonzero mask `0x3` | [x] |
| C35 | `matrixsum` | parameter zero/nonzero mask `0x4` | [x] |
| C36 | `matrixsum` | parameter zero/nonzero mask `0x5` | [x] |
| C37 | `matrixsum` | parameter zero/nonzero mask `0x6` | [x] |
| C38 | `matrixsum` | parameter zero/nonzero mask `0x7` | [x] |
| C39 | `matrixsum` | parameter zero/nonzero mask `0x8` | [x] |
| C40 | `matrixsum` | parameter zero/nonzero mask `0x9` | [x] |
| C41 | `matrixsum` | parameter zero/nonzero mask `0xA` | [x] |
| C42 | `matrixsum` | parameter zero/nonzero mask `0xB` | [x] |
| C43 | `matrixsum` | parameter zero/nonzero mask `0xC` | [x] |
| C44 | `matrixsum` | parameter zero/nonzero mask `0xD` | [x] |
| C45 | `matrixsum` | parameter zero/nonzero mask `0xE` | [x] |
| C46 | `matrixsum` | parameter zero/nonzero mask `0xF` | [x] |
