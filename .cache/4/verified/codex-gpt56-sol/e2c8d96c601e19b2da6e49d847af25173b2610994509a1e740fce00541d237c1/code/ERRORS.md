# Error Surface

The source scan covered `c_src/include/lib.h` and `c_src/src/lib.c` for error
returns, `NULL`, assertions, range checks, enums, and min/max constants.
`rev16` accepts one `uint32_t` by value and has no rejection or error path.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|

Checked error rows: **0 / 0**

Generic pointer, length, and enum boundary cases are not applicable because
the public API has no pointers, lengths, or enum parameters. Zero and
`UINT32_MAX` are valid scalar inputs and are covered by `CONFIGS.md`.

