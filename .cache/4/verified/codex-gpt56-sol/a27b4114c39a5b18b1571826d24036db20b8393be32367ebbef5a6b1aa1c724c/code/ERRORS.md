# Error Surface

The complete C source was scanned for `RETURN_ERROR`, negative and null
returns, assertions, enums, null checks, explicit range checks, and min/max
constants. It contains no explicit rejection or error branch.

| # | function | trigger (the exact invalid input/condition) | expected C result | Test |
|---|----------|---------------------------------------------|-------------------|------|

Generic FFI boundaries are not applicable: neither public function accepts a
pointer, length, enum, count, or documented bounded value. `driver` accepts the
entire C `int` domain. The unchecked `scanf` matching-failure and EOF outcomes
of `main` are observable valid configurations and are listed in `CONFIGS.md`.
