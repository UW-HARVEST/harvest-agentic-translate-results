# Error surface

Mechanically derived from null checks, explicit error returns, assertions,
range checks, and min/max constants in `../c_src/src` and
`../c_src/include`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `printLine` | `line == NULL` | [x] Returns `void` without writing any bytes to stdout. |

No error-return statements, assertions, public length parameters, public
enums, or explicit public range checks exist in the C source.
