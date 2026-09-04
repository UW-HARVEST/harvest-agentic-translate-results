# Configuration-surface table

Mechanically derived axes from `c_src/src/lib.c`:

- exported entry points/data: all eight symbols listed in `SYMBOLS.md`;
- dynamic-array shapes: capacity zero/one/many, size below capacity, and size at
  capacity (which selects the expansion branch);
- flag options: the full cross-product of READ/WRITE/EXECUTE/DELETE, plus
  unrelated high bits (which the C code ignores);
- matrix shapes: original contents and mutable zero/positive/negative values;
- `matrixsum` options: the full cross-product of each argument being zero or
  nonzero, with randomized nonzero magnitudes/signs.

Randomized rows use a fixed seed and avoid signed-overflow expressions in C.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `matrix` | read the initial 3x4 exported object byte-for-byte | [x] |
| C2 | `init_array` | capacity `0` (empty boundary; compare nullness and fields if allocated) | [x] |
| C3 | `init_array`, `free_array` | capacity `1` | [x] |
| C4 | `init_array`, `free_array` | randomized capacity `2..256` | [x] |
| C5 | `expand_array` | positive capacity `1`, preserving existing element data | [x] |
| C6 | `expand_array` | randomized positive capacity `2..128`, preserving all data | [x] |
| C7 | `add_element` | empty array with spare capacity (`size == 0 < capacity`) | [x] |
| C8 | `add_element` | nonempty array with spare capacity (`0 < size < capacity`) | [x] |
| C9 | `add_element`, `expand_array` | one-element full array (`size == capacity == 1`) | [x] |
| C10 | `add_element`, `expand_array` | randomized many-element full array (`size == capacity`, `2..64`) | [x] |
| C11 | `free_array` | allocated empty array | [x] |
| C12 | `free_array` | allocated populated array | [x] |
| C13 | `process_flags` | recognized mask `0x0` (none), randomized unrelated bits | [x] |
| C14 | `process_flags` | recognized mask `0x1` (READ), randomized unrelated bits | [x] |
| C15 | `process_flags` | recognized mask `0x2` (WRITE), randomized unrelated bits | [x] |
| C16 | `process_flags` | recognized mask `0x3` (READ+WRITE), randomized unrelated bits | [x] |
| C17 | `process_flags` | recognized mask `0x4` (EXECUTE), randomized unrelated bits | [x] |
| C18 | `process_flags` | recognized mask `0x5` (READ+EXECUTE), randomized unrelated bits | [x] |
| C19 | `process_flags` | recognized mask `0x6` (WRITE+EXECUTE), randomized unrelated bits | [x] |
| C20 | `process_flags` | recognized mask `0x7` (READ+WRITE+EXECUTE), randomized unrelated bits | [x] |
| C21 | `process_flags` | recognized mask `0x8` (DELETE), randomized unrelated bits | [x] |
| C22 | `process_flags` | recognized mask `0x9` (READ+DELETE), randomized unrelated bits | [x] |
| C23 | `process_flags` | recognized mask `0xA` (WRITE+DELETE), randomized unrelated bits | [x] |
| C24 | `process_flags` | recognized mask `0xB` (READ+WRITE+DELETE), randomized unrelated bits | [x] |
| C25 | `process_flags` | recognized mask `0xC` (EXECUTE+DELETE), randomized unrelated bits | [x] |
| C26 | `process_flags` | recognized mask `0xD` (READ+EXECUTE+DELETE), randomized unrelated bits | [x] |
| C27 | `process_flags` | recognized mask `0xE` (WRITE+EXECUTE+DELETE), randomized unrelated bits | [x] |
| C28 | `process_flags` | recognized mask `0xF` (all four), randomized unrelated bits | [x] |
| C29 | `calculate_matrix_checksum`, `matrix` | original matrix contents | [x] |
| C30 | `calculate_matrix_checksum`, `matrix` | all-zero matrix | [x] |
| C31 | `calculate_matrix_checksum`, `matrix` | randomized mixed positive/negative 3x4 matrix with in-range sum | [x] |
| C32 | `matrixsum` | zero/nonzero mask `0x0`; randomized matrix | [x] |
| C33 | `matrixsum` | zero/nonzero mask `0x1`; randomized signs/magnitudes and matrix | [x] |
| C34 | `matrixsum` | zero/nonzero mask `0x2`; randomized signs/magnitudes and matrix | [x] |
| C35 | `matrixsum` | zero/nonzero mask `0x3`; randomized signs/magnitudes and matrix | [x] |
| C36 | `matrixsum` | zero/nonzero mask `0x4`; randomized signs/magnitudes and matrix | [x] |
| C37 | `matrixsum` | zero/nonzero mask `0x5`; randomized signs/magnitudes and matrix | [x] |
| C38 | `matrixsum` | zero/nonzero mask `0x6`; randomized signs/magnitudes and matrix | [x] |
| C39 | `matrixsum` | zero/nonzero mask `0x7`; randomized signs/magnitudes and matrix | [x] |
| C40 | `matrixsum` | zero/nonzero mask `0x8`; randomized signs/magnitudes and matrix | [x] |
| C41 | `matrixsum` | zero/nonzero mask `0x9`; randomized signs/magnitudes and matrix | [x] |
| C42 | `matrixsum` | zero/nonzero mask `0xA`; randomized signs/magnitudes and matrix | [x] |
| C43 | `matrixsum` | zero/nonzero mask `0xB`; randomized signs/magnitudes and matrix | [x] |
| C44 | `matrixsum` | zero/nonzero mask `0xC`; randomized signs/magnitudes and matrix | [x] |
| C45 | `matrixsum` | zero/nonzero mask `0xD`; randomized signs/magnitudes and matrix | [x] |
| C46 | `matrixsum` | zero/nonzero mask `0xE`; randomized signs/magnitudes and matrix | [x] |
| C47 | `matrixsum` | zero/nonzero mask `0xF`; randomized signs/magnitudes and matrix | [x] |
| C48 | `init_array`, `add_element`, `expand_array`, `free_array` | end-to-end low-level sequence from capacity `1`, multiple expansions | [x] |
| C49 | `init_array`, `add_element`, `expand_array`, `free_array` | end-to-end low-level sequence from capacity `2`, the same growth path used by `matrixsum` | [x] |
| C50 | `init_array`, `add_element`, `free_array` | end-to-end low-level sequence with exact capacity (no expansion) | [x] |
| C51 | all function exports and `matrix` | repeated mixed operations while keeping C and Rust state synchronized | [x] |

Feature surface: `Cargo.toml` declares no features. The sole effective
configuration is therefore the empty/default feature set; verification runs
both normal commands and `--no-default-features` to prove they are identical.
