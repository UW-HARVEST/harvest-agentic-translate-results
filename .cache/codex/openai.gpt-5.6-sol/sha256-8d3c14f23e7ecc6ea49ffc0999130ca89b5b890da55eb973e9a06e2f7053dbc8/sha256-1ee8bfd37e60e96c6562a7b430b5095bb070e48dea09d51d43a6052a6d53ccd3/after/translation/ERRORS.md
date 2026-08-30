# Error Surface

Mechanical searches covered `RETURN_ERROR`, negative and null returns,
`assert`, explicit range/null checks, error enums, and min/max constants in
`../c_src/include/driver.h` and `../c_src/src/driver.c`.

The sole API is `void driver(int x)`. It has no error result, pointer or enum
arguments, assertions, rejection branches, explicit range checks, or error
sentinels. Negative and zero values are accepted and produce no output, so
they are valid configurations recorded in `CONFIGS.md`, not errors.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

## Completeness

- [x] Distinct C rejection branches: 0.
- [x] Error rows requiring differential tests: 0.
- [x] Generic pointer, length, and enum boundary cases: not applicable to this API.
