# Error Surface

Mechanical searches covered `return`, `assert`, `if`, `switch`, `case`,
`NULL`, `ERROR`, `MIN`, `MAX`, enums, and preprocessor conditionals in
`../c_src/include` and `../c_src/src`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|

No rows exist. `driver(int x)` accepts every value representable by the C
`int` parameter and returns `void`. The public API has no pointers, lengths,
enums, ranges, sentinels, assertions, or explicit rejection paths.
