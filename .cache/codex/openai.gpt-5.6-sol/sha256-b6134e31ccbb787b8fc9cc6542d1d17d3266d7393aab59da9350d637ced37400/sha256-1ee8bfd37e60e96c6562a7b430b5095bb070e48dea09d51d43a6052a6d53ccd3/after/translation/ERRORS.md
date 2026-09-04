# Error Surface

Mechanical searches covered `RETURN_ERROR`, negative and null returns,
`assert`, `if`, `switch`, preprocessor conditionals, `NULL`, enums, and
min/max constants in `src/driver.c` and `include/driver.h`.

The C implementation contains no explicit rejection branch, error return,
assertion, range check, null check, length parameter, enum parameter, or
min/max constant. Consequently, the required rejection table has zero rows.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|

Generic FFI boundaries that exist despite the absence of C rejection logic:

| # | function | boundary | expected C behavior | tested |
|---|----------|----------|---------------------|--------|
| G1 | `foo` | `in == NULL`, with non-NUL `c` | process terminates from invalid memory access in `strchr`; no error sentinel exists | [x] |
| G2 | `driver` | `in == NULL` | process terminates while evaluating the first `foo` call; no error sentinel exists | [x] |
| G3 | `foo` | empty NUL-terminated input | valid call; returns `0` | [x] |
| G4 | `driver` | empty NUL-terminated input | valid call; writes `A: 0\nx: 0\n` | [x] |

There are no lengths to set to zero or oversize, no documented numeric range,
and no enum-valued argument for an out-of-range enum test.
