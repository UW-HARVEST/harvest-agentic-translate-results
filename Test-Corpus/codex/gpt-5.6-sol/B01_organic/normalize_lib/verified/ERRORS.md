# Error Surface

The mechanical scan covered `c_src/include/lib.h` and `c_src/src/lib.c` for
error-return statements/macros, `assert`, null checks, range checks, enums, and
min/max constants. The C API has one `void` function and contains none of those
constructs, so there are **zero source-derived rejection rows**.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

## Generic FFI Boundaries

These are required boundary probes rather than explicit C rejection branches.
Signal results describe the built default C shared object; dereferencing null
or supplying a length beyond the allocation is outside the C API's defined
memory contract.

| # | function | boundary | expected C result | status |
|---|----------|----------|-------------------|--------|
| B1 | `normalize` | `dest == src == NULL`, `size == 0` | returns normally; no memory access | [x] |
| B2 | `normalize` | `dest == src`, `size == -1` | returns normally; both loops and `memset` are skipped | [x] |
| B3 | `normalize` | `dest == src`, `size == INT_MIN` | returns normally; both loops and `memset` are skipped | [x] |
| B4 | `normalize` | null source with positive size | process-level fault from source dereference | [x] |
| B5 | `normalize` | null destination, nonzero source, positive size | process-level fault from destination write | [x] |
| B6 | `normalize` | distinct pointers with `size == -1` | process-level fault from the C conversion of `size * sizeof(float)` to a huge `memset` length | [x] |
| B7 | `normalize` | `size == INT_MAX` with null buffers | process-level fault on the first source access; covers the oversized-length boundary | [x] |

There are no documented numeric ranges or enum parameters, so no one-past-range
or invalid-enum case exists for this API.
