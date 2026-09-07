# Error surface

Mechanical scan inputs:

```text
../c_src/include/lib.h
../c_src/src/lib.c
```

The scan covered `RETURN_ERROR`, `return -1`, `return NULL`, `assert`,
error enums, null/range checks, and min/max constants. The C source contains
no explicit rejection or error-return path: `merge_sort` returns `void` and
unconditionally calls `memcpy` before recursing.

## Source-level rejection table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

Source-level rejection rows: **0**.

## Mandatory generic FFI boundary coverage

These are not C rejection branches. They record the compiled C library's
observable process behavior so undefined invalid-pointer/length calls can be
compared without crashing the test runner.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| G1 [x] | `merge_sort` | `a = NULL`, `b = NULL`, `size = 0` | returns normally (child status 0) |
| G2 [x] | `merge_sort` | `a = NULL`, valid `b`, `size = 1` | terminated by `SIGSEGV` (shell status 139) |
| G3 [x] | `merge_sort` | valid `a`, `b = NULL`, `size = 1` | terminated by `SIGSEGV` (shell status 139) |
| G4 [x] | `merge_sort` | `a = NULL`, `b = NULL`, `size = -1` | terminated by `SIGSEGV` (shell status 139) |
| G5 [x] | `merge_sort` | `a = NULL`, `b = NULL`, `size = INT_MAX` | terminated by `SIGSEGV` (shell status 139) |

There are no public enum parameters and no documented bounded value range, so
out-of-range enum and one-past-documented-range cases are not applicable.
