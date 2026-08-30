# Error Surface

Mechanical search covered `RETURN_ERROR`, negative returns, `NULL` returns,
assertions, `if`, `switch`, preprocessor conditionals, enums, `NULL`, and
min/max constants in `../c_src/include` and `../c_src/src`.

The library has no error codes, error enums, assertions, range checks, length
parameters, or enum parameters. Its only input rejection is the null-pointer
guard below.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| [x] 1 | `printLine` | `line == NULL` | returns `void` without writing output |
