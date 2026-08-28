# Error Surface

Mechanically derived from every `if`, `switch`, `return`, null/range check,
assertion, and error-like sentinel in `c_src/src/lib.c`. The C source has no
error macros, assertions, pointer checks, length parameters, min/max input
constants, or error enums. Its collision comparisons return ordinary boolean
results and are listed as valid configurations in `CONFIGS.md`.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `c2Collided` | `typeB` is any integer other than `0`, `1`, or `2`; `-1` and `3` are the one-step boundary values. The default branch does not inspect `A` or `B`, including when either is null. | `0` | [x] |

Null pointers with a valid `typeB` are dereferenced by C and have undefined
behavior, not a C rejection result. No table row claims a result for undefined
behavior. The only exported header API, `circle_collide`, has no pointer or
length arguments.
