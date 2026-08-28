# Error Surface

Mechanical search covered `c_src/src/lib.c` and `c_src/include/lib.h` for
error-return macros/statements, `assert`, null checks, explicit range checks,
error enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

The C source contains no explicit rejection or error path. `tfm` returns
`void`; when `count <= 0`, it performs no pointer access and leaves the
destination unchanged. Pointer/length combinations that do not designate the
required accessible storage when `count > 0` invoke undefined C behavior and
therefore have no C rejection result to compare.

## Generic Boundary Applicability

| boundary | coverage |
|----------|----------|
| Null pointers | Covered with zero and negative counts, the combinations for which C does not dereference them |
| Zero length | Covered |
| Large length | Covered with 16,384 triples and correctly sized storage |
| One past a documented range | Not applicable: the API documents no numeric range |
| Out-of-range enum | Not applicable: the API has no enum parameter |
| Positive count with null/short storage | Not comparable: C behavior is undefined rather than an error result |
