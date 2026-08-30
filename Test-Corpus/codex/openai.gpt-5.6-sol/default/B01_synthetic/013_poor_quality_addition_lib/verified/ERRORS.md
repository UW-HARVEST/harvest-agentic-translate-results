# Error Surface

The C implementation has no error return codes, error enums, assertions, range
checks, or length parameters. Its one explicit rejection is the null check in
`printLine`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `printLine` | `line == NULL` | [x] Returns `void` without writing any bytes to stdout |

Generic FFI boundaries were also reviewed mechanically:

- `printLine`: `NULL` is row 1; an empty C string is valid and covered in
  `CONFIGS.md`; there is no length, enum, or numeric range.
- `printIntLine`: every value representable by C `int` is valid; there is no
  pointer, length, enum, or narrower documented range.
- `bad`, `good`, and `driver`: no parameters.
