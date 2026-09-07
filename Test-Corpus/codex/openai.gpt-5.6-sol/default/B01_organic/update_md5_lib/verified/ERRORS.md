# Error surface

Mechanical scan inputs:

```text
../c_src/include/lib.h
../c_src/src/lib.c
```

Patterns inspected include error-return statements/macros, `assert`, null
checks, range comparisons, enums, and min/max constants.

## Explicit C rejections

| # | function | trigger (the exact invalid input/condition) | expected C result | Status |
|---|----------|----------------------------------------------|-------------------|--------|

There are no explicit rejection branches, error returns, assertions, null
checks, enums, documented ranges, or length parameters in the C source.

## Generic FFI boundary probes

These are required generic probes, not explicit C rejection branches. Because
the C implementation dereferences these pointers without checking them, each
probe is isolated in a subprocess and compares the exact process termination
status.

| # | function | trigger (the exact invalid input/condition) | expected C result | Status |
|---|----------|----------------------------------------------|-------------------|--------|
| G1 | `tflac_pack_u64le` | `d == NULL` | process terminates with the platform access-violation signal | [x] |
| G2 | `tflac_md5_addsample` | `m == NULL` | process terminates with the platform access-violation signal | [x] |
| G3 | `update_md5` | `t == NULL`, valid `samples` | process terminates with the platform access-violation signal | [x] |
| G4 | `update_md5` | valid `t`, `samples == NULL` | process terminates with the platform access-violation signal | [x] |

Zero/oversized lengths and one-past-range enum values are not applicable:
none of the three exported functions accepts a length or enum parameter.
