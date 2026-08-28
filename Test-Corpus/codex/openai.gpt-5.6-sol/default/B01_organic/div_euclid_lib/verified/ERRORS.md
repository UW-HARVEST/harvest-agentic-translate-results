# Error Surface

Mechanically derived by searching `c_src/include/` and `c_src/src/` for
returns, conditionals, assertions, null checks, range checks, error constants,
and min/max constants. The scalar-only API has no pointer, length, or enum
inputs.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| [x] 1 | `div_euclid` | `v2 == 0` | returns the sentinel `0` |
