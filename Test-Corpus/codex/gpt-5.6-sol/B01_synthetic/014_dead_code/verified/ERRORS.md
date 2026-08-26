# Error Surface

Mechanical search covered `RETURN_ERROR`, negative and null returns, error
enums, `assert`, range/min/max checks, null checks, `if`, and `switch` in every
C source file. The sole rejection branch is the null guard below.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `printLine` | `line == NULL` | returns `void` without writing any bytes | [x] |

There are no length parameters, enum parameters, error returns, assertions,
range constants, or other rejection branches in the C source.
