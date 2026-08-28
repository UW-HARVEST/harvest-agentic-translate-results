# Error Surface

Mechanical searches covered `RETURN_ERROR`, `return -1`, `return NULL`,
`assert`, `if`, `switch`, preprocessor conditionals, null checks, range checks,
and min/max constants in all C sources and public headers.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|

There are no rejection paths. `max_size_frame` takes three `uint32_t` values
by value and accepts the full domain of each argument. It has no pointers,
length contracts, enums, sentinels, assertions, or error returns. Zero and
`UINT32_MAX` are valid values and are covered as valid-path boundaries in
`CONFIGS.md`.

- [x] Every C rejection branch is represented (the source contains none).
