# Error surface

Mechanical searches covered `c_src/src/lib.c` and `c_src/include/lib.h` for
error-return statements/macros, assertions, null checks, explicit range checks,
enums, and min/max constants.

The C implementation has **no explicit rejection paths**: it returns `void`,
contains no error code or sentinel, and performs no validation.

| # | function | trigger (the exact invalid input/condition) | expected C result | Test |
|---|----------|----------------------------------------------|-------------------|------|

## Mandatory generic FFI boundaries

These are boundary cases required by Phase C even though they are not explicit
C rejection branches.

| # | function | boundary | expected C behavior | Test |
|---|----------|----------|---------------------|------|
| B1 | `tfm` | `count == 0`, including null `dest` and `src` | returns normally without dereferencing either pointer | [x] |
| B2 | `tfm` | `count < 0`, including `INT_MIN` and null pointers | returns normally without dereferencing either pointer | [x] |
| B3 | `tfm` | `count == 1`, null `dest`, non-null `src` | undefined behavior; compare isolated-process termination behavior | [x] |
| B4 | `tfm` | `count == 1`, non-null `dest`, null `src` | undefined behavior; compare isolated-process termination behavior | [x] |
| B5 | `tfm` | oversized positive count with storage for only one item | undefined behavior; compare isolated-process termination behavior with guarded pages | [x] |

There are no enum parameters, documented numeric option ranges, error enums,
or feature-specific error paths.
