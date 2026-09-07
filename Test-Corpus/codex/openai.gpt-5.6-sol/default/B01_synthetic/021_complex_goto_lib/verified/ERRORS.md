# Error surface

Mechanical source scan covered `../c_src/include/driver.h` and
`../c_src/src/driver.c` for error-return statements/macros, assertions, null
checks, range checks, enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|---|---|---|:---:|
| — | — | No rejection or error path exists in the C API. | — | — |

`driver` takes two by-value C `int` values and returns `void`. There are no
pointers, lengths, enum parameters, documented ranges, sentinels, error codes,
assertions, or input-validation branches.

Generic FFI boundaries that apply are represented in the tests:

- zero values;
- `INT_MIN` values on terminating paths;
- negative values on terminating paths.

Null pointers, zero/oversized lengths, and out-of-range enum discriminants are
not applicable to this API. `INT_MAX` is a valid scalar but requires billions
of output operations, so randomized tests cover the same positive-value
branches at bounded sizes.

The input shape `x > 0 && y < 0` does not reach a normal return before the C
expression `y--` incurs signed-integer underflow. It is outside the C language's
defined, terminating behavior and therefore is not an error/rejection row.

## Completion

- [x] Every C rejection row is covered (the source contains zero such rows).
- [x] Applicable generic FFI scalar boundaries pass differential testing.
