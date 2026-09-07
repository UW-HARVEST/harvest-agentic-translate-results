# Error Surface

Mechanical audit scope:

```text
../c_src/include/lib.h
../c_src/src/lib.c
```

The source contains no pointers, lengths, enums, assertions, error enums,
`RETURN_ERROR` macros, `return -1`, or `return NULL` statements. It has one
explicit invalid-input rejection:

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `div_euclid` | `v2 == 0`, for every possible `int v1` | returns integer `0` | [x] |

Generic FFI boundary categories that do not apply to this scalar-only API:
null pointers, zero/oversized lengths, and out-of-range enum discriminants.
