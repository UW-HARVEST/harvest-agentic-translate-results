# Error Surface

Mechanical searches covered `return -1`, `return NULL`, `RETURN_ERROR`,
`assert`, `if`, `switch`, `case`, `NULL`, enum declarations, range constants,
and preprocessor conditionals. `c_src/src/container_of.c` contains no explicit
rejection, error return, assertion, range check, null check, enum, or length
parameter. Therefore the source-derived rejection table has zero rows.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

## Generic FFI Boundaries

These mandatory boundaries are unchecked by the C source and are tested
differentially even though they are not source rejection branches.

| # | function | boundary | expected C result | |
|---|----------|----------|-------------------|-|
| G1 | `find_container_of_a` | `i == NULL` | returns `NULL` without dereferencing it | [x] |
| G2 | `find_container_of_b` | `i == NULL` | returns pointer value `NULL - offsetof(struct test, b)` without dereferencing it | [x] |
| G3 | `main` | `argv == NULL` | process terminates with `SIGSEGV` while evaluating `argv[1]` | [x] |
| G4 | `main` | valid `argv`, but `argv[1] == NULL` | process terminates with `SIGSEGV` in `atoi(NULL)` | [x] |
| G5 | `main` | valid `argv[1]`, but `argv[2] == NULL` | process terminates with `SIGSEGV` in `atoi(NULL)` | [x] |

Zero and oversized lengths do not apply because no exported function accepts a
length. Out-of-range enum values do not apply because no enum crosses the FFI
boundary.
