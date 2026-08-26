# Error Surface

Mechanically searched `c_src/src/staticalias.c` and
`c_src/include/staticalias.h` for error returns, null returns, assertions,
enums, range checks, null checks, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are no rejection branches in the C source, so the error-surface table
has zero rows.

`static_alias` unconditionally dereferences `outer`; null, misaligned,
read-only, or dangling pointers are outside the C function's defined domain
and are not rejected. Signed overflow in either addition is likewise undefined
C behavior, not an error result. `driver` accepts every `int` value; an
`iterations` value less than or equal to zero produces no calls and no output.

