# Configuration Surface

The public dynamic ABI consists of the eight entries in `SYMBOLS.md`. The
runtime axes in the C source are array occupancy/growth, four independent
permission bits, mutable matrix values, and zero/nonzero truthiness of each of
the four `matrixsum` parameters. Unknown flag bits are ignored.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|-------------------------------------------|----------|
| 1 | `init_array`, `free_array` | positive capacity `1`; empty array | [x] |
| 2 | `init_array`, `free_array` | positive capacity greater than `1`; empty array | [x] |
| 3 | `add_element` | `size < capacity`; append without growth | [x] |
| 4 | `add_element` | append makes `size == capacity`; no growth yet | [x] |
| 5 | `add_element`, `expand_array` | `size == capacity`; append triggers one doubling | [x] |
| 6 | `add_element`, `expand_array` | many elements trigger repeated doublings | [x] |
| 7 | `expand_array` | direct expansion of a valid nonempty-capacity array | [x] |
| 8 | `free_array` | null pointer; no operation | [x] |
| 9 | `free_array` | nonnull initialized array; data and object released | [x] |
| 10 | `matrix` | direct 48-byte object read and randomized write/read | [x] |
| 11 | `calculate_matrix_checksum` | compiled-in 3-by-4 matrix values | [x] |
| 12 | `matrix`, `calculate_matrix_checksum` | randomized signed `int[3][4]` values | [x] |
| 13 | `process_flags` | known-bit mask `0x0`; randomized unknown bits | [x] |
| 14 | `process_flags` | known-bit mask `0x1`; randomized unknown bits | [x] |
| 15 | `process_flags` | known-bit mask `0x2`; randomized unknown bits | [x] |
| 16 | `process_flags` | known-bit mask `0x3`; randomized unknown bits | [x] |
| 17 | `process_flags` | known-bit mask `0x4`; randomized unknown bits | [x] |
| 18 | `process_flags` | known-bit mask `0x5`; randomized unknown bits | [x] |
| 19 | `process_flags` | known-bit mask `0x6`; randomized unknown bits | [x] |
| 20 | `process_flags` | known-bit mask `0x7`; randomized unknown bits | [x] |
| 21 | `process_flags` | known-bit mask `0x8`; randomized unknown bits | [x] |
| 22 | `process_flags` | known-bit mask `0x9`; randomized unknown bits | [x] |
| 23 | `process_flags` | known-bit mask `0xA`; randomized unknown bits | [x] |
| 24 | `process_flags` | known-bit mask `0xB`; randomized unknown bits | [x] |
| 25 | `process_flags` | known-bit mask `0xC`; randomized unknown bits | [x] |
| 26 | `process_flags` | known-bit mask `0xD`; randomized unknown bits | [x] |
| 27 | `process_flags` | known-bit mask `0xE`; randomized unknown bits | [x] |
| 28 | `process_flags` | known-bit mask `0xF`; randomized unknown bits | [x] |
| 29 | `matrixsum` | zero/nonzero parameter mask `0x0`; randomized matrix | [x] |
| 30 | `matrixsum` | zero/nonzero parameter mask `0x1`; randomized matrix | [x] |
| 31 | `matrixsum` | zero/nonzero parameter mask `0x2`; randomized matrix | [x] |
| 32 | `matrixsum` | zero/nonzero parameter mask `0x3`; randomized matrix | [x] |
| 33 | `matrixsum` | zero/nonzero parameter mask `0x4`; randomized matrix | [x] |
| 34 | `matrixsum` | zero/nonzero parameter mask `0x5`; randomized matrix | [x] |
| 35 | `matrixsum` | zero/nonzero parameter mask `0x6`; randomized matrix | [x] |
| 36 | `matrixsum` | zero/nonzero parameter mask `0x7`; randomized matrix | [x] |
| 37 | `matrixsum` | zero/nonzero parameter mask `0x8`; randomized matrix | [x] |
| 38 | `matrixsum` | zero/nonzero parameter mask `0x9`; randomized matrix | [x] |
| 39 | `matrixsum` | zero/nonzero parameter mask `0xA`; randomized matrix | [x] |
| 40 | `matrixsum` | zero/nonzero parameter mask `0xB`; randomized matrix | [x] |
| 41 | `matrixsum` | zero/nonzero parameter mask `0xC`; randomized matrix | [x] |
| 42 | `matrixsum` | zero/nonzero parameter mask `0xD`; randomized matrix | [x] |
| 43 | `matrixsum` | zero/nonzero parameter mask `0xE`; randomized matrix | [x] |
| 44 | `matrixsum` | zero/nonzero parameter mask `0xF`; randomized matrix | [x] |
