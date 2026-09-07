# Error surface

Mechanical searches covered `c_src/include/` and `c_src/src/` for error-return
macros, `return -1`, `return NULL`, enums, assertions, null checks, explicit
`if`/`switch` checks, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

There are no defined rejection branches in the C source. `bitwriter_add`
unconditionally returns `0` after dereferencing `bw`. The header documents no
valid range for `bits`.

## Generic ABI boundary coverage

These are required boundary probes, not rows in the mechanically derived error
table because the C implementation does not reject them:

- [x] null `bw` pointer: matching external-process termination
- [x] zero `bits`: matching return value and all struct bytes
- [x] `bits == 65` (one past the 64-bit value width): matching observed ABI behavior
- [x] `bits == UINT32_MAX`: matching observed ABI behavior
- [x] out-of-range enum: not applicable; the public API has no enum parameter
- [x] zero/oversized buffer length: not applicable; `len` is state only and is not read
