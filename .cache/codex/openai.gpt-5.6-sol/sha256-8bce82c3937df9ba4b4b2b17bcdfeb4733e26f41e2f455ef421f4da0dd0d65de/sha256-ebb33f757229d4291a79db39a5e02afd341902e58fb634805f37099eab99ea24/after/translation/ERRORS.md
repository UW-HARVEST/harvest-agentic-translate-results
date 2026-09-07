# Error-surface table

Mechanical search scope: `../c_src/include/*.h` and `../c_src/src/*.c`.
Searched for error returns, `assert`, explicit range/null checks, error enums,
and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|

There are no rows because the C library has no rejection or error branch.
`driver(float)` returns `void` and accepts the complete set of `float` object
representations.

Generic FFI error boundaries are not applicable: the sole public API has no
pointer, length, count, range-limited integer, or enum parameter. Zero,
infinities, subnormals, and all NaN payloads are valid by-value float inputs and
are covered by the valid-path differential test.

Phase C status: **[x] complete** (zero rejection rows; generic invalid-input
categories do not exist for this signature).
