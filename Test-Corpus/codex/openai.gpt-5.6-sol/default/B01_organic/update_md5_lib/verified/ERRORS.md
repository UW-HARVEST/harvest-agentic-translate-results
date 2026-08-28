# Error Surface

Mechanical scan scope: `../c_src/include/lib.h` and
`../c_src/src/lib.c`.

The scan covered `RETURN_ERROR`, `return -1`, `return NULL`, all other return
statements, `assert`, null checks, explicit range checks, error enums, and
min/max constants. The C source contains no rejection or error path. Its only
return statement returns the normal `update_md5` result.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| - | - | No explicit C rejection branches exist | - |

## Generic FFI Boundaries

These are required boundary probes, not entries in the error-surface table:

| # | entry point | boundary | status |
|---|-------------|----------|--------|
| G1 | `tflac_pack_u64le` | null destination pointer | [x] |
| G2 | `tflac_md5_addsample` | null context pointer | [x] |
| G3 | `update_md5` | null `tflac` pointer | [x] |
| G4 | `update_md5` | null samples pointer with non-null `tflac` | [x] |

The API has no length parameter, enum parameter, documented numeric range, or
error sentinel. Zero and oversized lengths and out-of-range enum values are
therefore not representable at this FFI boundary. Null pointers are
unconditionally dereferenced by C, so the probes compare process termination
rather than an error return.
