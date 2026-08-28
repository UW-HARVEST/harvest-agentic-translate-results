# Error Surface

This table is derived from every conditional rejection or sentinel-return
branch in `c_src/src/lib.c`. The source has no `RETURN_ERROR`, `return -1`,
`return NULL`, `assert`, explicit pointer validation, or error-enum return.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| E1 | `is_valid_operation` | `op_char == 0` | `false` | [x] |
| E2 | `is_valid_operation` | nonzero `op_char < '1'` | `false` | [x] |
| E3 | `is_valid_operation` | `op_char > '5'` | `false` | [x] |
| E4 | `divide_operation` | `b == 0` | `0` | [x] |
| E5 | `modulo_operation` | `b == 0` | `0` | [x] |

`perform_computation_with_history` dereferences both pointer arguments without
checking them. A null `history` or `history_count` therefore has undefined
behavior in C rather than a defined error result. `allocate_results` passes
zero, negative, and oversized counts directly to `calloc`; its only sentinel
is the allocator's null result.

