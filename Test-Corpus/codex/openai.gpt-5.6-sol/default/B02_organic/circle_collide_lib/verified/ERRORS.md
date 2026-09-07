# Error Surface

Mechanically derived from all explicit `return` branches, comparisons,
`switch` defaults, assertions, null checks, and range checks in
`c_src/src/lib.c`. There are no assertions, null checks, length parameters,
error macros, error enums, `return -1`, or `return NULL` statements.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E01 | `c2Collided` | `typeB` is not `C2_TYPE_CIRCLE` (0), `C2_TYPE_AABB` (1), or `C2_TYPE_CAPSULE` (2) | returns `0` without dereferencing `A` or `B` [x] |

## Generic FFI boundaries

- Null `A` or `B` with a valid `typeB` is not rejected by C; C dereferences the
  pointer, so the behavior is undefined. Differential tests run these cases in
  child processes and compare their observed termination behavior without
  risking the test harness.
- Null `A` and `B` with an invalid `typeB` is defined by E01 and returns `0`.
- Zero/oversized lengths do not apply: no public function accepts a length.
- One-past-enum and arbitrary C-enum integers are covered by E01, including
  `-1`, `3`, `INT_MIN`, and `INT_MAX`.
