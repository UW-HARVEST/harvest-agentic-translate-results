# Error-surface table

Derived from every `return 0` rejection branch and every pointer dereference in
`../c_src/src/lib.c`. This API has no lengths, allocation failures, assertions,
error enums, `NULL` returns, or `-1` returns.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `f2` | `typeA == C2_TYPE_CIRCLE` and `typeB` is neither `CIRCLE` nor `AABB` | returns `0` without dereferencing `B` | [x] |
| 2 | `f2` | `typeA == C2_TYPE_AABB` and `typeB` is neither `CIRCLE` nor `AABB` | returns `0` without dereferencing `B` | [x] |
| 3 | `f2` | `typeA` is neither `CIRCLE` nor `AABB` | returns `0` without dereferencing either pointer | [x] |
| 4 | `f3` | `v2 == 0` | returns `0` | [x] |
| 5 | `f2` | valid tags and `A == NULL` | process terminates by invalid pointer access | [x] |
| 6 | `f2` | valid tags and `B == NULL` | process terminates by invalid pointer access | [x] |
| 7 | `f4` | `rnd == NULL` | process terminates by invalid pointer access | [x] |
| 8 | `f11` | `dest == NULL` with valid `src` | process terminates by invalid pointer access | [x] |
| 9 | `f11` | `src == NULL` with valid `dest` | process terminates by invalid pointer access | [x] |
| 10 | `f12` | `dest == NULL` with valid `src` | process terminates by invalid pointer access | [x] |
| 11 | `f12` | `src == NULL` with valid `dest` | process terminates by invalid pointer access | [x] |
| 12 | `f13` | `dest == NULL` with valid `src` | process terminates by invalid pointer access | [x] |
| 13 | `f13` | `src == NULL` with valid `dest` | process terminates by invalid pointer access | [x] |

The null-pointer rows are generic FFI-boundary cases, not explicit C
rejections. Their differential tests run in child processes so either library
cannot terminate the test harness.
