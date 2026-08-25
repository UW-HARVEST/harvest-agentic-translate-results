# Error Surface

The C source has no assertions, error macros, error enums, length parameters,
allocation failures, or explicit numeric range rejection. It has four explicit
rejection branches. Rows 5-11 record the additionally mandated null-pointer
boundary behavior: C performs no null check in those paths, so the external
result is process termination rather than a returned sentinel.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `f2` | `typeA == C2_TYPE_CIRCLE` and `typeB` is neither `C2_TYPE_CIRCLE` nor `C2_TYPE_AABB` | returns `0` without reading `B` | [x] |
| 2 | `f2` | `typeA == C2_TYPE_AABB` and `typeB` is neither `C2_TYPE_CIRCLE` nor `C2_TYPE_AABB` | returns `0` without reading `B` | [x] |
| 3 | `f2` | `typeA` is neither `C2_TYPE_CIRCLE` nor `C2_TYPE_AABB` | returns `0` without reading `A` or `B` | [x] |
| 4 | `f3` | `v2 == 0` | returns `0` | [x] |
| 5 | `f2` | valid circle/circle types and `A == NULL` | unchecked dereference; process-fatal | [x] |
| 6 | `f2` | valid circle/circle types and `B == NULL` | unchecked dereference; process-fatal | [x] |
| 7 | `f4` | `rnd == NULL` | unchecked dereference; process-fatal | [x] |
| 8 | `f11` | `dest == NULL` or `src == NULL` | unchecked dereference; process-fatal | [x] |
| 9 | `f12` | `dest == NULL` or `src == NULL` | unchecked dereference; process-fatal | [x] |
| 10 | `f13` | `dest == NULL` or `src == NULL` | unchecked dereference; process-fatal | [x] |

Zero and oversized lengths are not applicable: no exported function accepts a
length. Zero scalar values remain valid inputs and are covered in
`CONFIGS.md`.
