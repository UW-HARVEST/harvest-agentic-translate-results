# Error Surface

Mechanical audit covered `c_src/include/staticloop.h` and
`c_src/src/staticloop.c` for error-return statements/macros, assertions, null
checks, explicit range checks, enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

There are no rejection branches or error sentinels in the C source, so the
error-surface table has zero rows.

## Generic FFI Boundary Audit

| Boundary category | Applicability |
|-------------------|---------------|
| Null pointers | Not applicable: neither public entry point accepts a pointer. |
| Zero lengths | Not applicable: neither public entry point accepts a length. |
| Oversized lengths | Not applicable: neither public entry point accepts a length. |
| Out-of-range enums | Not applicable: the public API declares no enum parameters. |
| One past documented range | Not applicable: both parameters are unrestricted C `int` values. |

## Phase C Verification

- [x] Every error-surface row has a passing differential test (zero rows).
- [x] Every generic FFI boundary category was audited and is not applicable to
  this pointer-free, length-free, enum-free API.
- [x] The audit holds for the default and `--no-default-features` builds.
