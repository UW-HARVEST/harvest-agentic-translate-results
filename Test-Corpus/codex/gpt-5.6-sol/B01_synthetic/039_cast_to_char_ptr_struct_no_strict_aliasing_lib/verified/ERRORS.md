# Error Surface

Mechanical searches covered `c_src/include/` and `c_src/src/` for error-return
macros, negative or null returns, assertions, conditionals, switches, null
checks, range checks, enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are **0 rejection branches**. `driver` returns `void`, accepts one
by-value C `int`, and contains no validation, pointer, length, enum, option,
range, or error-sentinel surface. Every value representable by C `int` is
accepted, so the generic null-pointer, zero/oversized-length, out-of-range
enum, and one-past-range cases are not applicable to this API.
