# Error Surface

Mechanically derived from every `return NULL`, `return -1`, null check, and
allocation check in `c_src/src/lib.c`. The source contains no assertions,
enums, explicit numeric range checks, or error macros.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `allocate_block` | `malloc(sizeof(MemoryBlock))` returns `NULL` (`!mb`) | returns `NULL` | [x] |
| 2 | `allocate_block` | `calloc(count, sizeof(int))` returns `NULL` (`!mb->data`) | frees `mb`, then returns `NULL` | [x] |
| 3 | `betagamma` | either internal allocation is `NULL` (`!mem1 \|\| !mem2`), reachable for `param1` values whose `(param1 % 10) + 5` converts to an oversized `size_t` | frees both internal block pointers, then returns `-1` | [x] |

The pointer checks in `free_block` accept `NULL` as a valid no-op and are
covered in `CONFIGS.md`. Passing `NULL` as `create_block.name` or either
`compute_hash` argument is not rejected by C; it has undefined behavior by
passing the pointer to `strcpy` or dereferencing it. Differential subprocess
tests cover the observable process-level behavior without crashing the test
runner.
