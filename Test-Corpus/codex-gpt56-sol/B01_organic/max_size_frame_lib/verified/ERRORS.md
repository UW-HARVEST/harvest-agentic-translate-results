# Error Surface

The public API consists only of:

```c
uint32_t max_size_frame(uint32_t blocksize, uint32_t channels, uint32_t bitdepth);
```

Mechanical inspection of `c_src/include/lib.h` and `c_src/src/lib.c` finds no
error-return macro, error enum, sentinel return, assertion, range check, null
check, pointer, length, or enum parameter. Every possible value of each
`uint32_t` parameter is accepted, including zero and `UINT32_MAX`.

| # | function | trigger (the exact invalid input/condition) | expected C result | Tested |
|---|----------|----------------------------------------------|-------------------|--------|
| - | - | No rejection conditions exist | - | [x] |

Consequently Phase C has no rejection rows. Scalar zero and oversized boundary
values are valid inputs and are covered by the valid-path rows in
`CONFIGS.md`.
