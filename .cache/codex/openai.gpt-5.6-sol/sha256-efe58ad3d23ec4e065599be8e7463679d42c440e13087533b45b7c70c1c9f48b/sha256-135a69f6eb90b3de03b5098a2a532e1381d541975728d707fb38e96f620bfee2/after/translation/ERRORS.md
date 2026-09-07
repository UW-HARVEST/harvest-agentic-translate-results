# Error-surface table

Mechanical search covered all `if`, `switch`, `case`, `default`, `return`,
`assert`, null/range/min/max/error macro patterns in `c_src/src`.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|---|
| 1 | executable `main` | `argc < 3` (fewer than two user arguments) | writes `usage: <argv[0]> A B\n` to stderr and exits with status `2` | [x] |

The shared-library API has no rejection paths: its exported functions accept
only by-value C `int` arguments and return C `int`. There are no pointer,
length, enum, allocation, null, assertion, or explicit range-error inputs.
`use_generated(n)` treats values outside `0..=6` as a valid default branch and
returns the selected operation's initial accumulator; those cases are in
`CONFIGS.md`, not this error table.
