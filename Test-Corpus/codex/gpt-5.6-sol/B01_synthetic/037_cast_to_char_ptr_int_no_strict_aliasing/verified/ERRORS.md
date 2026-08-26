# Error Surface

Mechanical search scope: `c_src/src/main.c`.

Searched for error returns, `assert`, null checks, range checks, error enums,
and min/max constants. The C source contains none. `main` ignores the return
value from `scanf`, initializes `x` to zero, always calls `driver`, and always
returns zero.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

There are zero explicit C rejection branches, so the error-surface table has
zero rows to check. Generic FFI boundaries still require tests for failed input
conversion, EOF, and values outside the `int` range.

Generic boundary completion:

- [x] Failed `%d` matching and embedded NUL input match.
- [x] EOF before assignment matches.
- [x] `INT_MAX + 1`, `INT_MIN - 1`, and oversized decimal input match.
- [x] Null-pointer cases are not applicable: neither public function accepts a pointer.
- [x] Zero/oversized lengths are not applicable: neither public function accepts a length.
- [x] Out-of-range enums are not applicable: neither public function accepts an enum.

These are covered by `phase_b_main_matching_failure`,
`phase_b_main_input_failure_at_eof`, and
`phase_c_generic_invalid_and_range_boundaries`.
