# Error Surface

Mechanical scans of `c_src/src/lib.c` and `c_src/include/lib.h` found no
`RETURN_ERROR`, `return -1`, `return NULL`, error enum, `assert`, explicit
range check, or null check. `bitwriter_add` has only `return 0`, so the C
library has no defined rejection rows.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

## Mandatory generic FFI boundaries

These are not C rejection branches. They are tracked because Phase C requires
generic ABI boundary probes even when the source provides no error handling.

| # | function | boundary | expected C result | [ ] |
|---|----------|----------|-------------------|-----|
| G1 | `bitwriter_add` | `bw == NULL` | No C rejection; dereference has undefined behavior. Compare child-process termination. | [x] |
| G2 | `bitwriter_add` | `bits == 0` | No C rejection; the initial shift is undefined in C. Compare the built C shared object's observed return value and output bytes. | [x] |
| G3 | `bitwriter_add` | `bits > 64` (including `65` and `UINT32_MAX`) | No C rejection; shifts can be undefined in C. Compare the built C shared object's observed return value and output bytes. | [x] |

There are no enum parameters, documented length parameters, or documented
input ranges in the public header.
