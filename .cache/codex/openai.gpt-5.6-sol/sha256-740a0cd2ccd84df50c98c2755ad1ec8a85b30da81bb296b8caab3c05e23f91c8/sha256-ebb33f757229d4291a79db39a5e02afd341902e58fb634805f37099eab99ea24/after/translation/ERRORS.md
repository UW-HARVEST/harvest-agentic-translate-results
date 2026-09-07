# Error surface

Mechanical source scan covered `RETURN_ERROR`, negative/sentinel returns,
`NULL`, `assert`, `if`, `switch`, `default`, range checks, and min/max tokens in
`../c_src/include/lib.h` and `../c_src/src/lib.c`.

There are no error macros, assertions, pointer checks, length arguments, range
checks, or error enums. The only defined rejection behavior is the three
`default` branches in `collided`. A null pointer with a valid type is
dereferenced by C and is outside the C function's defined behavior; null
pointers are safely testable only with a rejected type combination that returns
before dereferencing.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `collided` | `typeA == C2_TYPE_CIRCLE` (`0`) and `typeB` is any integer other than `0` or `1`; neither pointer is dereferenced | [x] return `0` |
| 2 | `collided` | `typeA == C2_TYPE_AABB` (`1`) and `typeB` is any integer other than `0` or `1`; neither pointer is dereferenced | [x] return `0` |
| 3 | `collided` | `typeA` is any integer other than `0` or `1`, for every `typeB`; neither pointer is dereferenced | [x] return `0` |

Generic boundary audit:

- Null pointers: covered in all three rows where C does not dereference them.
- Zero/oversized lengths: not applicable; no exported function accepts a
  length.
- One-past-range enums: rows 1-3 include `2`, plus negative and extreme
  `int` values.
