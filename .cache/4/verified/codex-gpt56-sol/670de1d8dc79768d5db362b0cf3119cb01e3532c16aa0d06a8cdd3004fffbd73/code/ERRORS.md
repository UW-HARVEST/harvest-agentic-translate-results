# Error Surface

The rejection rows below come from every explicit error return in
`c_src/src/lib.c`. There are no assertions, error enums, explicit scalar range
checks, or public enum parameters in the C source.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `allocate_block` | `malloc(sizeof(MemoryBlock)) == NULL` | returns `NULL` | [x] |
| 2 | `allocate_block` | `calloc(count, sizeof(int)) == NULL`; `SIZE_MAX` is a deterministic overflowing request on the test platform | frees the allocated `MemoryBlock` and returns `NULL` | [x] |
| 3 | `betagamma` | `!mem1 || !mem2`; naturally reached when `param1 % 10` is `-9..=-6`, making `(param1 % 10) + 5` negative before conversion to `size_t`, or by an injected allocation failure | frees either non-null block and returns `-1` | [x] |

## Generic FFI Boundaries

These rows record the mandatory generic boundaries even where the C code does
not reject them. Fatal cases are run in isolated child processes so the
observed C and Rust signals can be compared.

| # | function | boundary | expected C behavior | tested |
|---|----------|----------|---------------------|--------|
| 4 | `create_block` | `name == NULL` | process terminates with `SIGSEGV` while `strcpy` reads `name` | [x] |
| 5 | `create_block` | 32-byte name plus terminator, one byte beyond the 31-byte payload capacity | returns a block with all 32 name bytes copied and the requested flag; C performs an out-of-bounds terminator write before assigning `flags` | [x] |
| 6 | `allocate_block` | `count == 0` | returns a block of size zero when the platform's `calloc(0, ...)` returns non-null | [x] |
| 7 | `allocate_block` | `count == SIZE_MAX` | returns `NULL` through rejection row 2 | [x] |
| 8 | `free_block` | `mb == NULL` | returns normally without freeing | [x] |
| 9 | `free_block` | `mb != NULL` and `mb->data == NULL` | frees only `mb` and returns normally | [x] |
| 10 | `compute_hash` | `mb1 == NULL` | process terminates with `SIGSEGV` while reading `mb1->data` | [x] |
| 11 | `compute_hash` | `mb2 == NULL` with non-null `mb1` | process terminates with `SIGSEGV` while reading `mb2->data` | [x] |
| 12 | all entry points | out-of-range enum value | not applicable: the C API has no enum parameters | [x] |
