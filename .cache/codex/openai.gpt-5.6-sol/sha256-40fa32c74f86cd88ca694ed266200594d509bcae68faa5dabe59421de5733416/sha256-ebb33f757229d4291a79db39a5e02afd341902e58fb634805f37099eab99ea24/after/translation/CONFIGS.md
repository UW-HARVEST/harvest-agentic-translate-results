# Configuration surface

Mechanically derived from the exported symbols, the four flag branches, the
four parameter-truthiness branches, the dynamic-array capacity branch, and the
fixed 3-by-4 matrix loops in `../c_src/src/lib.c`.

There are no Cargo features, C preprocessor configuration branches, runtime
mode setters, or executable targets. The only runtime option axes are the four
flag bits / four parameter truth values, mutable matrix contents, and dynamic
array size-versus-capacity shape.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `init_array`, `free_array` | positive capacity `1`; returned size `0`, capacity `1` | [x] |
| 2 | `init_array`, `free_array` | positive capacity `2` (the `matrixsum` path); returned size `0`, capacity `2` | [x] |
| 3 | `init_array`, `free_array` | positive capacity greater than `2`; randomized small capacities | [x] |
| 4 | `expand_array` | non-null array with `size < capacity`; capacity doubles and existing elements survive | [x] |
| 5 | `expand_array` | non-null array with `size == capacity`; capacity doubles and all elements survive | [x] |
| 6 | `add_element` | `size < capacity`; append without expansion | [x] |
| 7 | `add_element`, `expand_array` | `size == capacity`; append after successful expansion | [x] |
| 8 | `init_array`, `add_element`, `free_array` | capacity `1`, empty/one/many sequence crossing multiple growth boundaries | [x] |
| 9 | `init_array`, `add_element`, `free_array` | capacity `2`, four-element sequence used by `matrixsum` | [x] |
| 10 | `free_array` | non-null initialized array containing randomized elements | [x] |
| 11 | `process_flags` | low flag mask `0b0000` (none) with randomized unrelated high bits | [x] |
| 12 | `process_flags` | low flag mask `0b0001` (read) with randomized unrelated high bits | [x] |
| 13 | `process_flags` | low flag mask `0b0010` (write) with randomized unrelated high bits | [x] |
| 14 | `process_flags` | low flag mask `0b0011` (read+write) with randomized unrelated high bits | [x] |
| 15 | `process_flags` | low flag mask `0b0100` (execute) with randomized unrelated high bits | [x] |
| 16 | `process_flags` | low flag mask `0b0101` (read+execute) with randomized unrelated high bits | [x] |
| 17 | `process_flags` | low flag mask `0b0110` (write+execute) with randomized unrelated high bits | [x] |
| 18 | `process_flags` | low flag mask `0b0111` (read+write+execute) with randomized unrelated high bits | [x] |
| 19 | `process_flags` | low flag mask `0b1000` (delete) with randomized unrelated high bits | [x] |
| 20 | `process_flags` | low flag mask `0b1001` (read+delete) with randomized unrelated high bits | [x] |
| 21 | `process_flags` | low flag mask `0b1010` (write+delete) with randomized unrelated high bits | [x] |
| 22 | `process_flags` | low flag mask `0b1011` (read+write+delete) with randomized unrelated high bits | [x] |
| 23 | `process_flags` | low flag mask `0b1100` (execute+delete) with randomized unrelated high bits | [x] |
| 24 | `process_flags` | low flag mask `0b1101` (read+execute+delete) with randomized unrelated high bits | [x] |
| 25 | `process_flags` | low flag mask `0b1110` (write+execute+delete) with randomized unrelated high bits | [x] |
| 26 | `process_flags` | low flag mask `0b1111` (all four) with randomized unrelated high bits | [x] |
| 27 | `matrix`, `calculate_matrix_checksum` | compiled default 3-by-4 matrix contents | [x] |
| 28 | `matrix`, `calculate_matrix_checksum` | externally mutated 3-by-4 matrix with randomized signed values | [x] |
| 29 | `matrixsum` | parameter truth mask `0b0000`; all four parameters zero | [x] |
| 30 | `matrixsum` | parameter truth mask `0b0001`; only parameter 1 nonzero | [x] |
| 31 | `matrixsum` | parameter truth mask `0b0010`; only parameter 2 nonzero | [x] |
| 32 | `matrixsum` | parameter truth mask `0b0011`; parameters 1-2 nonzero | [x] |
| 33 | `matrixsum` | parameter truth mask `0b0100`; only parameter 3 nonzero | [x] |
| 34 | `matrixsum` | parameter truth mask `0b0101`; parameters 1 and 3 nonzero | [x] |
| 35 | `matrixsum` | parameter truth mask `0b0110`; parameters 2-3 nonzero | [x] |
| 36 | `matrixsum` | parameter truth mask `0b0111`; parameters 1-3 nonzero | [x] |
| 37 | `matrixsum` | parameter truth mask `0b1000`; only parameter 4 nonzero | [x] |
| 38 | `matrixsum` | parameter truth mask `0b1001`; parameters 1 and 4 nonzero | [x] |
| 39 | `matrixsum` | parameter truth mask `0b1010`; parameters 2 and 4 nonzero | [x] |
| 40 | `matrixsum` | parameter truth mask `0b1011`; parameters 1, 2, and 4 nonzero | [x] |
| 41 | `matrixsum` | parameter truth mask `0b1100`; parameters 3-4 nonzero | [x] |
| 42 | `matrixsum` | parameter truth mask `0b1101`; parameters 1, 3, and 4 nonzero | [x] |
| 43 | `matrixsum` | parameter truth mask `0b1110`; parameters 2-4 nonzero | [x] |
| 44 | `matrixsum` | parameter truth mask `0b1111`; all four parameters nonzero | [x] |

