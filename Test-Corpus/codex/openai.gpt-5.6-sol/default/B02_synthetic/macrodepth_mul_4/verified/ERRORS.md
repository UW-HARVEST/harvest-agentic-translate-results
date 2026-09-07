# Error-surface table

This table was derived by scanning all of `c_src/` for error returns,
`RETURN_ERROR`, `return -1`, `return NULL`, assertions, null checks, explicit
range checks, min/max constants, enums, and conditional branches.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `main` | `argc < 3` | Write `usage: <argv[0]> A B\n` to `stderr` and return `2` without reading `argv[1]` or `argv[2]`. | [x] |

No other C function rejects input. The exported arithmetic/helper functions
take only `int` values and contain no pointer, length, enum, or explicit range
validation. Signed-overflow inputs are outside the defined C input domain and
are not classified as rejection paths.

Final verification: the row passed under all 2,048 Cargo feature subsets.
Generic zero/negative/oversized `argc`, runtime `n=7` and larger, and null
`argv` crash parity were also covered; there are no enum or length APIs.
