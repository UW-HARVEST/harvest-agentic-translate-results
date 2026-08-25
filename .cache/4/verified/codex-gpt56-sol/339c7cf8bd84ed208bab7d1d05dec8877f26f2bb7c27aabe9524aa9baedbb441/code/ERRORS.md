# Error Surface

Mechanically derived from null checks, range checks, assertions, explicit
error returns, and error enums in `c_src/`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `printLine` | `line == NULL` | Return `void` without writing any bytes to stdout | [x] |

No C entry point accepts a length or enum. The source contains no assertions,
range checks, min/max constants, error enums, or explicit error-return
statements.
