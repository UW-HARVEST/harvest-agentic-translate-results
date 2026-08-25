# Error Surface

Mechanical scans covered `c_src/include/lib.h` and `c_src/src/lib.c` for error
returns, `NULL`, assertions, enums, explicit range checks, and min/max
constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

There are no rows: `rgb_to_hsv` returns `void`, has no rejection branch, does
not validate either pointer, and accepts no lengths, enums, modes, or bounded
integer values.

## Required ABI Boundary Probes

These are not C error-surface rows because the C implementation does not
reject them. They are tracked separately to cover the generic null-pointer
boundary requirement.

| # | function | boundary | C contract | [ ] |
|---|----------|----------|------------|-----|
| N1 | `rgb_to_hsv` | `dest == NULL`, valid `src` | undefined behavior (observed `SIGSEGV`) | [x] |
| N2 | `rgb_to_hsv` | valid `dest`, `src == NULL` | undefined behavior (observed `SIGSEGV`) | [x] |

Zero and oversized lengths are not representable in this API. There is no enum
parameter and therefore no out-of-range enum value to pass across the FFI
boundary.
