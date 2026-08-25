# Error Surface

The C source contains no error-return macro, `return -1`, `return NULL`,
assertion, explicit range check, null check, or enum validation. Its only
input-rejection branch is `scanf("%d", &data[i]) != 1` at
`c_src/src/main.c:43`.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `main` | `scanf` returns `EOF` before the first conversion (empty or whitespace-only input) | Stop scanning, call `driver(data, 0)`, emit no bytes, return `0` | [x] |
| 2 | `main` | `scanf` returns `0` before the first conversion (first non-whitespace byte cannot begin `%d`) | Leave the byte unread, stop scanning, call `driver(data, 0)`, emit no bytes, return `0` | [x] |
| 3 | `main` | `scanf` returns `EOF` after one or more successful conversions | Stop at the successful-prefix length, emit exactly that prefix's transformed values, return `0` | [x] |
| 4 | `main` | `scanf` returns `0` after one or more successful conversions | Stop at the successful-prefix length, emit exactly that prefix's transformed values, return `0` | [x] |

There are no C rejection results for `fma_array` or `driver`. A null pointer is
not dereferenced when `len <= 0`; a null pointer with `len > 0` invokes
undefined behavior rather than a C-defined error path.

Generic boundary probes also pass for null pointers at non-positive lengths,
positive-length null pointers (isolated and compared by termination signal),
`INT_MIN` length, oversized valid arrays, and the 101st `main` input. There are
no enum inputs.
