# ERRORS.md — Phase C error-surface table

Mechanically derived from every rejection / early-return / sentinel in the C
source. Grep basis:

```sh
grep -n 'return\|assert\|if (\|NULL\|<\|>' c_src/src/simplestruct.c c_src/include/simplestruct.h
```

The C library contains exactly ONE rejection construct: the `else return -1;`
branch guarded by `if (head)`. There are no `assert`s, no error enums, no
range checks, no min/max constants, and no other `return` of a sentinel.

| # | function | trigger (exact invalid input/condition) | expected C result | test | status |
|---|----------|------------------------------------------|-------------------|------|--------|
| 1 | `smallestValue` | `head == NULL` (the `if (head)` guard fails, `else return -1`) | returns `-1` | `err_row1_null_head` | [x] |

## Generic FFI boundary cases (required even though not in the table above)

| # | function | trigger | expected C result | test | status |
|---|----------|---------|-------------------|------|--------|
| G1 | `smallestValue` | NULL pointer (same as row 1, asserted independently and repeatedly) | `-1` | `err_row1_null_head` | [x] |
| G2 | `smallestValue` | single node, `value == -1` — sentinel collision: valid result indistinguishable from the NULL error | `-1` (NOT an error) | `err_sentinel_collision_minus_one` | [x] |
| G3 | `smallestValue` | list length 0 handled only via NULL (no length parameter exists → no "zero length" other than NULL) | `-1` | `err_row1_null_head` | [x] |
| G4 | `smallestValue` | oversized / extreme values: `INT_MIN`, `INT_MAX` as node values (one step past nothing — full `int` range is valid) | the true minimum, incl. `INT_MIN` | `err_extreme_int_values` | [x] |
| G5 | `smallestValue` | `INT_MIN` present together with `-1` (checks no signed-comparison / sentinel confusion) | `INT_MIN` | `err_extreme_int_values` | [x] |
| G6 | `smallestValue` | very long list (10_000 nodes) — traversal depth / recursion-vs-loop divergence | true minimum | `err_long_list_no_stack_divergence` | [x] |
| G7 | `smallestValue` | no enum parameters exist in this API, so there is no out-of-range enum value to pass. Documented as N/A. | N/A | — | [x] N/A |
| G8 | `smallestValue` | misaligned / garbage non-NULL pointer | UNDEFINED BEHAVIOUR in C — deliberately NOT tested (would be UB in both, no defined result to compare) | UB | — | [x] excluded |

All rows tested in `tests/differential.rs` against BOTH `.so` files loaded via
`libloading`.
