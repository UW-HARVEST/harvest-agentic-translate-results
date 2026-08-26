# Error Surface

Mechanical search covered `c_src/src/lib.c` and `c_src/include/lib.h` for error
returns, `NULL`, assertions, range checks, enums, and min/max constants. The C
source contains no rejection or error path, so the error-surface table has no
rows.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

## FFI Boundaries

The generic boundaries are handled as follows:

| # | function | boundary | C behavior | [ ] |
|---|----------|----------|------------|-----|
| B1 | `stbds_hash_bytes` | `p == NULL`, `len == 0` | Valid: no dereference; returns the empty-input hash | [x] |
| B2 | `stbds_hash_bytes` | `p == NULL`, `len > 0` | Undefined behavior: dereferences `p`; there is no C rejection result | n/a |
| B3 | `stbds_hash_bytes` | `len` exceeds the pointed-to allocation | Undefined behavior: reads beyond the allocation; there is no C rejection result | n/a |
| B4 | both entry points | out-of-range enum | Not applicable: the API has no enum parameters | [x] |

Undefined-behavior rows cannot be compared for an error code or sentinel that
the C implementation does not provide. Tests cover the defined zero-length
null-pointer boundary and allocation-backed zero, boundary, and large lengths.
