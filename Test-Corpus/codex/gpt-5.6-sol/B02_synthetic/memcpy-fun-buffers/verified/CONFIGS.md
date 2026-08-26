# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` defines no
options or compile definitions. There is one valid feature combination:

| # | Cargo feature combination | C configuration | checked |
|---|---------------------------|-----------------|---------|
| 1 | no features (`--no-default-features`) | default | [x] |

## Runtime and Input Configurations

Rows are the source-distinguished valid branches and data shapes for every
exported entry point. Randomized tests vary byte values within each row.

| # | entry point(s) | configuration (options set + input shape) | covered |
|---|----------------|-------------------------------------------|---------|
| 1 | `calculate_checksum` | length `0` | [x] |
| 2 | `calculate_checksum` | length `1` | [x] |
| 3 | `calculate_checksum` | length `2..255`, including values that wrap `uint32_t` | [x] |
| 4 | `calculate_checksum` | boundary length `256` | [x] |
| 5 | `validate_buffer` | valid checksum; length `0` | [x] |
| 6 | `validate_buffer` | valid checksum; length `1..255` | [x] |
| 7 | `validate_buffer` | valid checksum; length `256` | [x] |
| 8 | `init_buffer_array`, `free_buffer_array` | capacity `1` and resulting empty array | [x] |
| 9 | `init_buffer_array`, `free_buffer_array` | capacity `2..100`; mutate `count` and storage | [x] |
| 10 | `free_buffer_array` | `NULL` | [x] |
| 11 | `buffer_copy` | source length `0` | [x] |
| 12 | `buffer_copy` | source length `1..255` | [x] |
| 13 | `buffer_copy` | source length `256` | [x] |
| 14 | `buffer_reverse` | length `0` early return | [x] |
| 15 | `buffer_reverse` | length `1` | [x] |
| 16 | `buffer_reverse` | length `2..255` | [x] |
| 17 | `buffer_reverse` | length `256` | [x] |
| 18 | `buffer_merge` | both sources empty | [x] |
| 19 | `buffer_merge` | only left source empty | [x] |
| 20 | `buffer_merge` | only right source empty | [x] |
| 21 | `buffer_merge` | both nonempty; left shorter/equal/longer | [x] |
| 22 | `buffer_merge` | combined boundary length `256` | [x] |
| 23 | `buffer_split` | empty source at position `0` | [x] |
| 24 | `buffer_split` | nonempty source at position `0` | [x] |
| 25 | `buffer_split` | nonempty source at an interior position | [x] |
| 26 | `buffer_split` | nonempty source at `source.length` | [x] |
| 27 | `buffer_interleave` | both sources empty | [x] |
| 28 | `buffer_interleave` | only left source empty | [x] |
| 29 | `buffer_interleave` | only right source empty | [x] |
| 30 | `buffer_interleave` | equal nonzero lengths | [x] |
| 31 | `buffer_interleave` | unequal lengths; left shorter/longer | [x] |
| 32 | `buffer_interleave` | combined boundary length `256` | [x] |
| 33 | `buffer_rotate` | empty buffer with any position | [x] |
| 34 | `buffer_rotate` | nonempty buffer with position `0` | [x] |
| 35 | `buffer_rotate` | positive position below length | [x] |
| 36 | `buffer_rotate` | positive position equal to or above length | [x] |
| 37 | `buffer_rotate` | negative position with magnitude below length | [x] |
| 38 | `buffer_rotate` | negative position with magnitude equal to or above length | [x] |
| 39 | `buffer_rotate` | length `1` | [x] |
| 40 | `buffer_conditional_copy` | `copy_matching=true`; no/some/all bytes match pattern | [x] |
| 41 | `buffer_conditional_copy` | `copy_matching=false`; no/some/all bytes match pattern | [x] |
| 42 | `buffer_copy_strided` | empty source with positive stride | [x] |
| 43 | `buffer_copy_strided` | stride `1` | [x] |
| 44 | `buffer_copy_strided` | `1 < stride < length` | [x] |
| 45 | `buffer_copy_strided` | stride equal to length | [x] |
| 46 | `buffer_copy_strided` | stride greater than length | [x] |
| 47 | `process_buffer_array(OP_COPY)` | count `1` and count `2..100` | [x] |
| 48 | `process_buffer_array(OP_REVERSE)` | count `1` and count `2..100`; empty/nonempty buffers | [x] |
| 49 | `process_buffer_array(OP_MERGE)` | even count; each consecutive pair fits | [x] |
| 50 | `process_buffer_array(OP_MERGE)` | odd count; final buffer remains unchanged | [x] |
| 51 | `process_buffer_array(OP_ROTATE)` | count `1..100`; zero/positive/negative parameter | [x] |
| 52 | `process_buffer_array(OP_CHECKSUM)` | count `1..100`; valid and mismatched checksums (both accepted) | [x] |
| 53 | `read_buffer` | length `0` | [x] |
| 54 | `read_buffer` | length `1..255`; input integers cast to `uint8_t` | [x] |
| 55 | `read_buffer` | length `256` | [x] |
| 56 | `write_buffer` | length `0`, `1..255`, and `256` exact text formatting | [x] |
| 57 | `main(OP_COPY)` | count at least `2`; first buffer empty/nonempty/boundary | [x] |
| 58 | `main(OP_REVERSE)` | count `1..100`; empty/one/many/boundary lengths | [x] |
| 59 | `main(OP_MERGE)` | count at least `2`; empty/unequal/boundary combined lengths | [x] |
| 60 | `main(OP_SPLIT)` | position `0`, interior, and end; empty/nonempty source | [x] |
| 61 | `main(OP_INTERLEAVE)` | count at least `2`; empty/equal/unequal/boundary combined lengths | [x] |
| 62 | `main(OP_ROTATE)` | count `1..100`; empty and nonempty; zero/positive/negative amount | [x] |
| 63 | `main(OP_CHECKSUM)` | count `1..100`; empty/one/many/boundary lengths | [x] |
| 64 | `process_buffer_array` | negative `count`: copy/reverse/rotate/checksum loops are skipped and return `0`; merge rejects | [x] |
