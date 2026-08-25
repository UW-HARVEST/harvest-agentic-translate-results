# Error Surface

Mechanical searches covered `return -1`, `return NULL`, `RETURN_ERROR`,
assertions, explicit null/range checks, enum defaults, and min/max constants.
This C source has no error codes, assertions, or explicit pointer/length
rejections. It has four defined rejection branches for invalid enum values.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|-|
| 1 | `c2Collided` | `typeA` is not `C2_TYPE_CIRCLE`, `C2_TYPE_AABB`, or `C2_TYPE_CAPSULE` | `0`; neither shape pointer is dereferenced | [x] |
| 2 | `c2Collided` | `typeA == C2_TYPE_CIRCLE` and `typeB` is outside the three enum values | `0`; `A` and `B` are not dereferenced | [x] |
| 3 | `c2Collided` | `typeA == C2_TYPE_AABB` and `typeB` is outside the three enum values | `0`; `A` and `B` are not dereferenced | [x] |
| 4 | `c2Collided` | `typeA == C2_TYPE_CAPSULE` and `typeB` is outside the three enum values | `0`; `A` and `B` are not dereferenced | [x] |

Null pointers to required objects and out-of-bounds counts/indices are undefined
C behavior, not rejection results, so differential calls cannot safely be made
for them. Defined null optional pointers in `c2GJK`, zero `c2Support` count with
a valid first element, zero divisors, and invalid `c2MakeProxy` enum values are
covered as valid configuration rows in `CONFIGS.md`.
