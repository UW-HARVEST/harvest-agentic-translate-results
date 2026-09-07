# Error Surface

Mechanical searches covered `RETURN_ERROR`, negative and null returns,
assertions, null checks, range checks, error enums, and min/max constants in
`c_src/src/` and `c_src/include/`.

The C library contains no input validation, error return, assertion, range
check, null check, error enum, or rejection branch. Therefore the
source-derived error-surface table has zero rows.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|

Null `dest` or `src` pointers are outside the C function's defined behavior;
Phase C nevertheless compares their observed process-level behavior as generic
FFI boundary probes. There are no length parameters or enum parameters in this
API, so zero/oversized lengths and out-of-range enum values are not applicable.

## Generic FFI boundary probes

| probe | result |
|---|---|
| null `src` | [x] C and Rust terminate identically |
| null `dest` | [x] C and Rust terminate identically |
