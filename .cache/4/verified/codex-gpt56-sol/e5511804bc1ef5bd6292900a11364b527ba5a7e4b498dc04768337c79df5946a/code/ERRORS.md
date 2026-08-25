# Error Surface

The scan covered all of `c_src/include/` and `c_src/src/` for error-return
statements/macros, assertions, error enums, explicit range checks, null checks,
and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are no explicit rejection branches in the C implementation. In
particular, neither exported function validates pointers or lengths. Inputs
that make C form an invalid VLA, dereference an invalid pointer, overflow
address arithmetic, or overflow the `len * sizeof(int)` size calculation have
undefined behavior rather than a C error result, so they cannot have a
byte-identical return sentinel.

Generic FFI boundary coverage required by Phase C:

- [x] Null pointers with a non-dereferencing zero length.
- [x] Zero length.
- [x] Negative `fma_array` length (the loop performs no iterations).
- [x] Large valid length (there is no documented or checked finite maximum).
- [x] One-past-range enum values: not applicable; the API has no enums.
- [x] One-past documented numeric range: not applicable; no numeric range is
  documented or checked.

Positive lengths paired with null/undersized pointers and negative `driver`
lengths invoke undefined behavior in C. They are excluded from differential
calls because the reference library specifies no result to match.
