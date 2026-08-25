# Error Surface

Rows are derived from every explicit rejection, propagated error return,
resource failure, null/range check, and invalid `switch` arm in
`c_src/src/main.c`. A checksum mismatch is included because it emits a warning,
although C deliberately accepts it.

| # | function | trigger (the exact invalid input/condition) | expected C result | covered |
|---|----------|---------------------------------------------|-------------------|---------|
| 1 | `validate_buffer` | `buf == NULL` | `false` | [x] |
| 2 | `validate_buffer` | `buf->length > 256` | `false` | [x] |
| 3 | `validate_buffer` | `buf->checksum != calculate_checksum(...)` | warning; `true` | [x] |
| 4 | `init_buffer_array` | `initial_capacity <= 0` | `NULL` | [x] |
| 5 | `init_buffer_array` | allocation of `buffer_array_t` fails | `NULL` | [x] |
| 6 | `init_buffer_array` | allocation of buffer storage fails | frees array; `NULL` | [x] |
| 7 | `buffer_copy` | `src == NULL || dst == NULL` | `-1` | [x] |
| 8 | `buffer_copy` | `validate_buffer(src) == false` | `-1` | [x] |
| 9 | `buffer_reverse` | `buf == NULL` | `-1` | [x] |
| 10 | `buffer_merge` | any of `src1`, `src2`, or `dst` is `NULL` | `-1` | [x] |
| 11 | `buffer_merge` | `src1->length + src2->length > 256` | `-1` | [x] |
| 12 | `buffer_split` | any of `src`, `dst1`, or `dst2` is `NULL` | `-1` | [x] |
| 13 | `buffer_split` | `split_pos > src->length` | `-1` | [x] |
| 14 | `buffer_interleave` | any of `src1`, `src2`, or `dst` is `NULL` | `-1` | [x] |
| 15 | `buffer_interleave` | `src1->length + src2->length > 256` | `-1` | [x] |
| 16 | `buffer_rotate` | `buf == NULL` | `-1` | [x] |
| 17 | `buffer_conditional_copy` | `src == NULL || dst == NULL` | `-1` | [x] |
| 18 | `buffer_copy_strided` | `src == NULL || dst == NULL` | `-1` | [x] |
| 19 | `buffer_copy_strided` | `stride <= 0` | `-1` | [x] |
| 20 | `process_buffer_array` | `arr == NULL || arr->count == 0` | `-1` | [x] |
| 21 | `process_buffer_array(OP_COPY)` | nested `buffer_copy` rejects the first buffer | `-1` | [x] |
| 22 | `process_buffer_array(OP_REVERSE)` | nested `buffer_reverse` rejects a buffer | `-1` | [x] |
| 23 | `process_buffer_array(OP_MERGE)` | `arr->count < 2` | `-1` | [x] |
| 24 | `process_buffer_array(OP_MERGE)` | nested `buffer_merge` rejects a pair | `-1` | [x] |
| 25 | `process_buffer_array(OP_ROTATE)` | nested `buffer_rotate` rejects a buffer | `-1` | [x] |
| 26 | `process_buffer_array(OP_CHECKSUM)` | nested `validate_buffer` rejects a buffer | `-1` | [x] |
| 27 | `process_buffer_array` | `op` is not `0`, `1`, `2`, `5`, or `6` (including enum values `3` and `4`) | `-1` | [x] |
| 28 | `read_buffer` | `buf == NULL` | `-1` | [x] |
| 29 | `read_buffer` | `scanf("%d", &length) != 1` | `-1` | [x] |
| 30 | `read_buffer` | `length < 0 || length > 256` | `-1` | [x] |
| 31 | `read_buffer` | a byte's `scanf("%d", &byte) != 1` | `-1` after preserving earlier writes | [x] |
| 32 | `write_buffer` | `buf == NULL` | returns `void` after error | [x] |
| 33 | `main` | operation `scanf` fails | returns `1` | [x] |
| 34 | `main` | buffer-count `scanf` fails | returns `1` | [x] |
| 35 | `main` | `buffer_count <= 0 || buffer_count > 100` | returns `1` | [x] |
| 36 | `main` | `init_buffer_array(buffer_count) == NULL` | returns `1` | [x] |
| 37 | `main` | nested `read_buffer` fails | frees array; returns `1` | [x] |
| 38 | `main(OP_COPY)` | `buffer_count < 2` | returns `1` | [x] |
| 39 | `main(OP_REVERSE)` | nested `buffer_reverse` fails | stops; returns `1` | [x] |
| 40 | `main(OP_MERGE)` | `buffer_count < 2` | returns `1` | [x] |
| 41 | `main(OP_MERGE)` | nested `buffer_merge` fails | returns `1` | [x] |
| 42 | `main(OP_SPLIT)` | split-position `scanf` fails | returns `1` | [x] |
| 43 | `main(OP_SPLIT)` | nested `buffer_split` rejects the position, including a negative `int` converted to `size_t` | returns `1` | [x] |
| 44 | `main(OP_INTERLEAVE)` | `buffer_count < 2` | returns `1` | [x] |
| 45 | `main(OP_INTERLEAVE)` | nested `buffer_interleave` fails | returns `1` | [x] |
| 46 | `main(OP_ROTATE)` | rotation-amount `scanf` fails | returns `1` | [x] |
| 47 | `main(OP_ROTATE)` | nested `buffer_rotate` fails | stops; returns `1` | [x] |
| 48 | `main` | operation is outside `0..=6` | returns `1` | [x] |

Rows 22, 25, 36, 39, and 47 are structurally present propagation/resource
branches that are not all reachable with a well-formed allocated array and the
preceding checks. Rows 22 and 25 are exercised through a null storage pointer;
row 36 is exercised with allocator fault injection. Rows 39 and 47 are proven
unreachable because `main` obtains non-null storage from
`init_buffer_array`, `read_buffer` bounds every length to `0..=256`, and the
only rejection in `buffer_reverse`/`buffer_rotate` is a null buffer. Their
successful non-null paths are covered for randomized inputs.
