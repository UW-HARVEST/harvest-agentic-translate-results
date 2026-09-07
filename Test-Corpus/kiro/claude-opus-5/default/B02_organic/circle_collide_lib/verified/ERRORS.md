# ERRORS.md — Phase C error / rejection surface table

Derived mechanically by grepping `c_src/src/lib.c` for every `return`, `if`,
`switch`/`case`/`default`, `assert`, null check, and named constant:

```
grep -n 'return\|assert\|NULL\|RETURN_ERROR\|if (\|switch\|default\|case ' src/lib.c include/lib.h
```

Findings from that grep, stated honestly:

* There is **no** `assert`, **no** `NULL` check, **no** `RETURN_ERROR` macro,
  **no** error enum, **no** `errno` use, and **no** `return -1` / `return NULL`
  anywhere in the library. No function has an out-of-band error channel: the
  `int`-returning collision functions return a *boolean* 0/1 predicate, and the
  `c2v`/`float` functions are total.
* The **only** true input-rejection branch in the whole library is
  `lib.c:112-113`, the `default:` arm of the `switch (typeB)` in `c2Collided`,
  which returns `0` for any `typeB` that is not one of the three enum values.
* All remaining "error-ish" behaviour is IEEE-754 degenerate-value behaviour
  (division by zero, NaN comparison, overflow to `inf`) that the C performs
  silently. Those are still real inputs whose results must match bit-for-bit,
  so they are enumerated below as rejection/edge rows.
* Passing `A == NULL` or `B == NULL` on a *valid* `typeB` dereferences the
  pointer in C (`*(c2Circle *)A`) — undefined behaviour, i.e. not a rejection
  the C defines. Rows 2-4 therefore test null only on the `default:` path, where
  the C provably never dereferences. This is noted rather than silently skipped.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `c2Collided` | `typeB == 3` (one past the last valid enum value `C2_TYPE_CAPSULE`) | `default:` arm → returns `0`, no deref of `A`/`B` | [x] |
| 2 | `c2Collided` | `typeB == -1` (negative out-of-range enum), `A`/`B` non-null | returns `0` | [x] |
| 3 | `c2Collided` | `typeB == INT_MIN` and `typeB == INT_MAX` (extreme out-of-range ints across the FFI boundary) | returns `0` | [x] |
| 4 | `c2Collided` | `typeB` out of range **and** `A == NULL, B == NULL` | returns `0` (C never dereferences on the `default:` arm) | [x] |
| 5 | `c2Collided` | every out-of-range `typeB` in `[-256, 256] \ {0,1,2}` (sweep, incl. 4, 100, 255) | returns `0` for all | [x] |
| 6 | `c2CircletoCircle` | `A.r`/`B.r` negative → `r2 = (A.r+B.r)^2` is non-negative, so a negative radius silently behaves like its absolute value | `d2 < r2` computed anyway; no rejection | [x] |
| 7 | `c2CircletoCircle` | `A.r + B.r` overflows to `+inf`, or `r2 = r2*r2` overflows to `+inf` (e.g. `r = 1e30`) | `d2 < inf` → `1` (unless `d2` is also `inf`/NaN) | [x] |
| 8 | `c2CircletoCircle` | any of `A.p`, `B.p`, `A.r`, `B.r` is NaN | every `<` comparison is false → returns `0` | [x] |
| 9 | `c2CircletoCircle` | `±inf` coordinates producing `inf - inf = NaN` in `c2Sub` | `d2` = NaN → `0` | [x] |
| 10 | `c2CircletoAABB` | inverted AABB (`B.min > B.max` componentwise) | no validation; `c2Clampv` = `Maxv(min, Minv(p,max))` yields `min`, result computed from that | [x] |
| 11 | `c2CircletoAABB` | degenerate AABB (`B.min == B.max`, zero area/point box) | no rejection; reduces to point-vs-circle | [x] |
| 12 | `c2CircletoAABB` | `A.r == 0` (zero radius) → `r2 == 0`, and `d2 < 0` is impossible | always returns `0` | [x] |
| 13 | `c2CircletoAABB` | NaN in `A.p`, `B.min`, `B.max`, or `A.r` | ternaries in `Maxv`/`Minv` take the *else* operand for NaN; `d2 < r2` false → `0` | [x] |
| 14 | `c2CircletoAABB` | `A.r = ±inf` → `r2 = inf`; or `A.r` overflow `1e30 * 1e30 = inf` | `d2 < inf` → `1` | [x] |
| 15 | `c2CircletoCapsule` | degenerate capsule `B.a == B.b` → `n = (0,0)` → `c2Dot(n,n) == 0` → **division by zero** `da / 0` | only reachable when `da >= 0 && db < 0`; with `da == db == 0` the `da<0` test is false and `db<0` is false, so the `else` (endpoint) branch runs — no division. `0.0/0.0` NaN is produced only if a NaN/inf makes `da>=0, db<0` hold | [x] |
| 16 | `c2CircletoCapsule` | `n != 0` but `da/dot(n,n)` overflows to `±inf` (huge `da`, tiny `dot(n,n)` from denormal `n`) | `c2Mulvs` yields `inf`/NaN → `d2` NaN → `d2 < r*r` false → `0` | [x] |
| 17 | `c2CircletoCapsule` | `A.r + B.r` negative (both radii negative) → `r*r` still `>= 0` | no rejection; behaves like `abs` | [x] |
| 18 | `c2CircletoCapsule` | NaN anywhere in `A`/`B` → `da < 0` false, `db < 0` false | takes the `bp` endpoint branch. NaN in `A.p`, `A.r`, `B.b` or `B.r` poisons `d2`/`r` → returns `0`; NaN in **`B.a` only** does NOT, because `bp = A.p - B.b` never reads `B.a`, so a hit (`1`) is still reported — verified against the C, not assumed | [x] |
| 19 | `c2CircletoCapsule` | `±inf` endpoints → `inf - inf = NaN` inside `c2Sub` | NaN propagates → `0` | [x] |
| 20 | `c2Dot` | `inf * 0` operands → NaN; `inf + (-inf)` → NaN | NaN returned bitwise-identically (quiet NaN) | [x] |
| 21 | `c2Mulvs` | `b = 0` with `a = ±inf` → NaN; `b = inf` with `a = 0` → NaN | NaN, sign/payload per x86 `mulss` (first source operand wins) | [x] |
| 22 | `c2Maxv` / `c2Minv` | either operand NaN (all four x/y NaN placements) | comparison false → **second** operand's component is returned (NOT NaN-quieting `fmax`/`fmin`) | [x] |
| 23 | `c2Maxv` / `c2Minv` | `a == b`, and `+0.0` vs `-0.0` (compare equal, so the `>`/`<` is false) | returns the `b` component → `-0.0` vs `+0.0` distinction is observable in the raw bits | [x] |
| 24 | `c2Clampv` | `lo > hi` (contradictory clamp bounds) | no validation; `Maxv(lo, Minv(a,hi))` = `lo` | [x] |
| 25 | `c2Clampv` | NaN in `a`, `lo`, or `hi` | ternary fallthrough, result is whichever operand the false comparison selects | [x] |
| 26 | `circle_collide` | `x`/`y`/`r` = NaN | all three sub-tests return `0` → result `0` | [x] |
| 27 | `circle_collide` | `r` negative, `r = ±inf`, `x`/`y` = `±inf` | no validation; result is the packed bitfield of the three predicates | [x] |
| 28 | `circle_collide` | `r = 0.0` / `-0.0` (degenerate zero-radius circle) | AABB test can never fire (row 12); circle/capsule tests use `r_sum` | [x] |
| 29 | `c2V` | `x`/`y` = NaN, signalling-NaN bit patterns, `±0.0`, denormals | stores the bits verbatim, no normalisation | [x] |
| 30 | all struct-by-value entry points | trap-representation / padding: `c2Circle` (12 B) and `c2Capsule` (20 B, MEMORY class) passed with garbage in tail padding | padding is not read; results must be unaffected | [x] |

All 30 rows are covered by `translation/tests/error_paths.rs` and pass against
both `.so`s (see Phase C section of the run log).
