# Error Surface

Mechanical search covered `RETURN_ERROR`, `return -1`, `return NULL`, `assert`,
all `if`/`switch` branches, null checks, and min/max-style constants in
`../c_src/src/driver.c` and `../c_src/include/driver.h`.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|---------------------------------------------|-------------------|-|
| 1 | `printLine` | `line == NULL` | Return `void` without writing any bytes to stdout | [x] |

Boundary audit: there are no length/count API parameters, documented numeric
ranges, enums, error enums, error-return macros, sentinel returns, assertions,
or public allocation results. Consequently, zero/oversized lengths,
one-past-range values, and out-of-range enum values are not applicable. The
only pointer-taking entry point and its null boundary are row 1.

The undersized `alloca(10)` in `bad` is intentional behavior on a parameterless
valid path, not an input rejection, and is covered in `CONFIGS.md`.
