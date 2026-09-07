# ERRORS.md — Phase A error-surface table

Derived mechanically from `c_src/src/lib.c`. Greps used:

```
grep -nE "return|assert|NULL|isnan|isinf|INT_MAX|INT_MIN|>=|<=|default:" c_src/src/lib.c
```

Findings: there are **no `assert`s**, **no error enums**, and **no `RETURN_ERROR`
macros** in this library. The complete set of rejection / saturation / sentinel
paths the C actually contains is enumerated below. Every early `return` that is
not the normal tail return counts as one row, as does every clamping branch and
the `switch` `default:` rejection.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test |
|----|----------|---------------------------------------------|-------------------|------|
| E1 | `safe_double_to_int` | `isnan(d)` — any NaN payload/sign (`lib.c:49`) | `0` | [x] |
| E2 | `safe_double_to_int` | `isinf(d) && d > 0` → `+INFINITY` (`lib.c:53`) | `INT_MAX` = `2147483647` | [x] |
| E3 | `safe_double_to_int` | `isinf(d) && !(d > 0)` → `-INFINITY` (`lib.c:53`) | `INT_MIN` = `-2147483648` | [x] |
| E4 | `safe_double_to_int` | finite `d >= (double)INT_MAX`, i.e. `d >= 2147483647.0` (`lib.c:57`) — includes exactly `2147483647.0`, `nextafter(2147483647.0, +inf)`, `1e300`, `DBL_MAX` | `INT_MAX` | [x] |
| E5 | `safe_double_to_int` | finite `d <= (double)INT_MIN`, i.e. `d <= -2147483648.0` (`lib.c:60`) — includes exactly `-2147483648.0`, `nextafter(-2147483648.0, -inf)`, `-1e300`, `-DBL_MAX` | `INT_MIN` | [x] |
| E6 | `allocate_and_compute` | `malloc(size * sizeof(DataPoint))` returns `NULL` (`lib.c:105`). `size` is an `int` widened to `size_t`, so any `size < 0` becomes a ~2^64 request that always fails. Also any positive `size` whose 16-byte request exceeds available memory. | `-1` | [x] |
| E7 | `allocate_and_compute` | `size <= 0` **and** malloc succeeds (only reachable for `size == 0`, where glibc `malloc(0)` returns a non-NULL 0-byte block): both loops are skipped, `sum` stays `0.0` | `0` | [x] |
| E8 | `allocate_and_compute` | accumulated `sum` saturates: `sum` reaches `>= (double)INT_MAX` / `<= (double)INT_MIN` / `+inf` / `-inf` / `NaN` and is funnelled through `safe_double_to_int` (`lib.c:121`). NaN arises for `multiplier = ±inf` because element 0 computes `0 * (0.0*inf)` = `0 * NaN` = `NaN`. | `INT_MAX` / `INT_MIN` / `INT_MAX` / `INT_MIN` / `0` respectively | [x] |
| E9 | `switch_fallthrough_calculator` | `operation` matches no `case`: any value outside `{0,1,2,3,4}` — negative, `5`, `INT_MIN`, `INT_MAX`, and out-of-range "enum-like" ints passed across FFI (`default:` at `lib.c:95`) | `0` (the input `value` is discarded) | [x] |
| E10 | `fallcalc` | `malloc(5 * sizeof(int))` returns `NULL` (`lib.c:145`) | `-1` | [x] (unreachable in practice — 20-byte request; documented + asserted non-`-1`) |
| E11 | `process_array_reverse` | `count <= 0` (loop guard `i < count` at `lib.c:71`) — no dereference occurs, so even a `NULL`/dangling `end` is accepted | `0` | [x] |
| E12 | `foreach_sum` | `count <= 0` (the `FOREACH` guard `keep && idx < size`, `lib.c:130`) — no dereference occurs, so a `NULL` `array` is accepted | `0` | [x] |

## Generic FFI boundary cases also covered in Phase C

These are not distinct C branches but are required by the task's "generic
boundaries" clause. Each is exercised in `tests/differential.rs`.

| #  | case | note | test |
|----|------|------|------|
| G1 | `process_array_reverse(NULL, 0)` | no deref; must return `0` on both sides | [x] |
| G2 | `foreach_sum(NULL, 0)` | no deref; must return `0` on both sides | [x] |
| G3 | `process_array_reverse(ptr, negative)` | loop never entered; `0` | [x] |
| G4 | `foreach_sum(ptr, negative)` | loop never entered; `0` | [x] |
| G5 | `allocate_and_compute` with `size = INT_MIN`, `-1`, `0`, `1` | `INT_MIN`/`-1` ⇒ `-1`; `0` ⇒ `0`; `1` ⇒ `0` | [x] |
| G6 | `switch_fallthrough_calculator` `operation` one step past each end of the valid range: `-1` and `5` | both `0` | [x] |
| G7 | out-of-range "enum" ints for `operation`: `INT_MIN`, `INT_MAX`, `6`, `255`, `0x8000_0000u32 as i32` | all `0` | [x] |
| G8 | `fallcalc` with each param at `INT_MIN` / `INT_MAX` / `0` / `-1` (full 4^k corner grid) | signed-overflow wrap parity | [x] |
| G9 | `fallcalc` where `param4 % 10 + 1` is negative (⇒ inner `-1`) vs `0` vs positive | exercises E6/E7 through the composed pipeline | [x] |
| G10 | `safe_double_to_int` on `-0.0`, subnormals, `nextafter` neighbours of both clamp bounds | truncation-toward-zero parity | [x] |
