# Error surface

Mechanically derived from every explicit null/error return and allocation check
in `../c_src/src/lib.c`. There are no assertions, error enums, explicit numeric
range checks, or public enum parameters.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| [x] E1 | `allocate_block` | `malloc(sizeof(MemoryBlock))` returns null | return null |
| [x] E2 | `allocate_block` | the struct allocation succeeds, then `calloc(count, sizeof(int))` returns null | free the struct and return null |
| [x] E3 | `betagamma` | either internal `allocate_block` call returns null (`!mem1 || !mem2`) | free both nullable block pointers and return `-1` |

Generic FFI boundaries required by Phase C, although they are not explicit C
rejection branches:

| # | function | boundary input | expected C behavior |
|---|----------|----------------|---------------------|
| [x] G1 | `create_block` | null `name` pointer | process receives the same fault as Rust while `strcpy` reads null |
| [x] G2 | `create_block` | empty, one-byte, and exactly 31-byte NUL-terminated names | accepted; returned bytes match |
| [x] G3 | `allocate_block` | zero count | match C's returned sentinel/allocated representation exactly |
| [x] G4 | `allocate_block` | oversized count (`SIZE_MAX`) | return null |
| [x] G5 | `free_block` | null block pointer | no-op |
| [x] G6 | `compute_hash` | null first block pointer | process receives the same fault as Rust |
| [x] G7 | `compute_hash` | null second block pointer | process receives the same fault as Rust |
| [x] G8 | `betagamma` | full `int` boundary values where C execution is defined | return value matches |
| [x] G9 | all APIs | out-of-range enum values | not applicable: the API has no enum parameters |
