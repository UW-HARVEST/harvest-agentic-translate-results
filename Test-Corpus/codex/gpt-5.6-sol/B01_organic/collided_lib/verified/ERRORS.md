# Error Surface

The C source has no error macros, assertions, null checks, length arguments,
range constants, or error enums. Its only explicit rejection behavior is the
three `default` branches in `collided`. A null pointer with a valid type is
unconditionally dereferenced by C and therefore is not a defined rejection
path. Null pointers are defined inputs only on the short-circuiting invalid-enum
paths below.

| # | function | trigger (the exact invalid input/condition) | expected C result | Covered |
|---|----------|----------------------------------------------|-------------------|---------|
| 1 | `collided` | `typeA` is any integer other than `C2_TYPE_CIRCLE` (0) or `C2_TYPE_AABB` (1), including -1 and 2; `A` and `B` may be null because neither is dereferenced | `0` | [x] |
| 2 | `collided` | `typeA == C2_TYPE_CIRCLE` and `typeB` is any integer other than 0 or 1, including -1 and 2; `A` and `B` may be null because neither is dereferenced | `0` | [x] |
| 3 | `collided` | `typeA == C2_TYPE_AABB` and `typeB` is any integer other than 0 or 1, including -1 and 2; `A` and `B` may be null because neither is dereferenced | `0` | [x] |

Generic boundary audit: there are no lengths, counts, documented numeric
ranges, or additional enum parameters. Valid-type null pointers invoke C
undefined behavior and have no C return value to compare.
