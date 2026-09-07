# Error Surface

Mechanical audit patterns included `RETURN_ERROR`, negative/sentinel returns,
`NULL`, assertions, explicit range/null checks, enums, and min/max constants
across `../c_src/include/` and `../c_src/src/`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

There are no rejection branches in the C source. `ldexp_q2` accepts a `float`
and an `int` by value, returns only a `float`, and has no pointers, lengths,
enums, error codes, sentinels, assertions, or documented valid range. Therefore
the error-surface table has zero rows. Signed exponent boundaries, including
negative values and both integer extrema, belong to the configuration surface
because the C function computes and returns a value rather than rejecting them.

Generic FFI-boundary audit:

- [x] Null pointers: not applicable; the API has no pointer parameters.
- [x] Zero/oversized lengths: not applicable; the API has no length parameters.
- [x] Out-of-range enums: not applicable; the API has no enum parameters.
- [x] One past a documented range: not applicable; no input range is documented.
- [x] Scalar boundaries: zero, negative values, `INT_MIN`, and `INT_MAX` are
  covered by the valid-path differential suite because C returns values for
  them rather than errors.
