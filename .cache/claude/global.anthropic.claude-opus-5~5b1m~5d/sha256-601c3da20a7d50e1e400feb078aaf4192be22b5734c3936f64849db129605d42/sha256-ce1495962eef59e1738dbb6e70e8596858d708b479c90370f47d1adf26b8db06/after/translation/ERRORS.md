# ERRORS.md — Error / rejection surface table (Phase A)

Derived **mechanically** from `c_src/src/lib.c` (33 lines). Every `return`,
every explicit comparison against a min/max constant, and every guard that
diverts control flow away from the ordinary arithmetic is listed as a row.

## Mechanical grep results

`c_src/src/lib.c` contains:

* `return` statements: 4 — lines 5, 10, 29, 31.
* early-out / rejection guards: 1 — `if (v2 == 0)` (line 4).
* min/max constant checks: 3 — `v2 != (-0x7fffffff - 1)` (lines 11, 24) and
  `v1 != (-0x7fffffff - 1)` (line 15), plus the same constant reached as the
  `else` fallbacks on lines 14, 21, 27.
* `assert` / `NULL` checks / error enums / `errno` / `RETURN_ERROR` macros: **0**
  (`grep -c 'assert\|NULL\|errno\|ERROR\|exit\|abort' c_src/src/lib.c` → 0).
* pointer parameters: **0** — the whole API is `int div_euclid(int, int)`, so
  there is no null-pointer or length/size surface to reject.

The only *value* the C uses as a rejection sentinel is the early `return 0` for
a zero divisor. The `INT_MIN` comparisons are explicit range checks that select
alternative arithmetic (they never fail the call), and the `r < 0` tail is a
correction branch. All of them are enumerated below because each is a distinct
way the C diverts from the nominal path, and each is a place a translation can
silently diverge.

`INT_MIN` is written in the C exactly as `(-0x7fffffff - 1)` = `-2147483648`;
`INT_MAX` = `2147483647`.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `div_euclid` | `v2 == 0` — divide-by-zero rejection guard, `c_src/src/lib.c:4`. Any `v1` (incl. `0`, `1`, `-1`, `INT_MAX`, `INT_MIN`). | `return 0` immediately (line 5); never touches `q`/`r`, never divides |
| 2 | `div_euclid` | `v1 >= 0 && v2 == INT_MIN` — range check `v2 != (-0x7fffffff - 1)` on line 11 **fails**, falling to line 14 | `q = 0, r = v1`; `r >= 0` (since `v1 >= 0`) so line 29 returns `q` = **`0`** |
| 3 | `div_euclid` | `v1 == INT_MIN` — range check `v1 != (-0x7fffffff - 1)` on line 15 **fails**, so `-v1` is never evaluated; control goes to the line 22/24/26 chain | one of rows 4/5/6 below; the ordinary `(-v1)` paths (lines 17, 19) are **not** taken |
| 4 | `div_euclid` | `v1 < 0 && v1 != INT_MIN && v2 == INT_MIN` — range check on line 18 **fails**, falling to line 21 | `q = 1`, then `r = v1 - q*v2 = v1 - INT_MIN` ∈ `[1, INT_MAX]` > 0, so line 29 returns **`1`** |
| 5 | `div_euclid` | `v1 == INT_MIN && v2 == INT_MIN` — range check on line 24 **fails**, falling to line 27 | `q = 1, r = 0`; `r >= 0` so line 29 returns **`1`** |
| 6 | `div_euclid` | `v1 == INT_MIN && v2 >= 1` — line 22/23 `INT_MIN`-safe rewrite `-(v1 + v2)` (avoids the trapping `-INT_MIN`) | `q = -((-(v1+v2))/v2) - 1`, `r = -((-(v1+v2))%v2)`; then tail row 9/10. e.g. `(INT_MIN, 1) -> INT_MIN`, `(INT_MIN, 2) -> INT_MIN/2 = -1073741824` |
| 7 | `div_euclid` | `v1 == INT_MIN && v2 < 0 && v2 != INT_MIN` — line 24/25 `INT_MIN`-safe rewrite `-(v1 - v2)` | `q = ((-(v1-v2))/(-v2)) + 1`, `r = -((-(v1-v2))%(-v2))`; then tail row 9/10 |
| 8 | `div_euclid` | `v1 == INT_MIN && v2 == -1` — **signed-overflow** sub-case of row 7: `-(v1-v2) = INT_MAX`, `INT_MAX/1 = INT_MAX`, then `q = INT_MAX + 1` overflows | at the `-O0` build used here the add wraps: `q = INT_MIN`, `r = -(INT_MAX % 1) = 0`, `r >= 0` so returns **`INT_MIN` (-2147483648)** |
| 9 | `div_euclid` | tail check `r >= 0` (line 28) true — the divisor divides exactly, or `r` came from rows 2/4/5 | `return q` unmodified (line 29) |
| 10 | `div_euclid` | tail check `r >= 0` **false** i.e. `r < 0` (only reachable from lines 17, 19, 23, 25), with `v2 > 0` | `return q + (-1)` = `q - 1` (line 31, ternary true arm) |
| 11 | `div_euclid` | tail check `r < 0` with `v2 < 0` (note the ternary tests `v2 > 0`, so `v2 == 0` can never reach here — row 1 already returned) | `return q + 1` (line 31, ternary false arm) |
| 12 | `div_euclid` | `v1 == 0` with any `v2 != 0` — degenerate numerator; takes line 10 (`v2 > 0`) or line 12 (`v2 < 0`) with `q = -0 = 0`, `r = 0` | `return 0` |

## Generic FFI-boundary cases (required even though not in the table above)

The C signature is `int div_euclid(int, int)`. There are **no pointers, no
lengths/sizes, and no enums** in this API, so the classic null-pointer /
zero-length / oversized-length / invalid-enum-variant probes do not exist as
distinct C code paths. They are still covered as follows, so that the "value
one step past the valid range" and "out-of-range enum" classes are not blind
spots:

| # | boundary class | how it is exercised |
|---|----------------|---------------------|
| G1 | null pointers | not applicable — no pointer parameter exists. Asserted by inspection of `include/lib.h`; the FFI signature loaded by the tests is `extern "C" fn(c_int, c_int) -> c_int`. |
| G2 | zero length / empty input | modelled by the scalar zeros: `v1 == 0`, `v2 == 0` (rows 1, 12), tested for both arguments. |
| G3 | oversized / out-of-domain value | `int` accepts its full 32-bit range, so **every** bit pattern is in-domain. The extremes `INT_MAX` and `INT_MIN` are tested for both arguments in all sign combinations. |
| G4 | one step past a documented range | the only ranges the C tests are `x != INT_MIN` and `x >= 0`. Both sides of each are probed: `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX`. |
| G5 | out-of-range "enum" value across FFI | no enum parameter exists; the moral equivalent — an `int` bit pattern with no corresponding valid case in the C's `if`/`else` ladder — is impossible because the ladder is total over `int`. Verified empirically by the exhaustive sweeps in Phase B, which cover **all** `2^32` values of `v2` for fixed boundary `v1`s and vice-versa via `full_axis_sweep`, plus 100 % of `[-512, 512]^2`. |
| G6 | signed-overflow / UB-adjacent input | row 8 (`INT_MIN, -1`) and the `INT_MIN` rewrites in rows 6/7 are the only overflow-capable expressions; each has a dedicated differential test. |
| G7 | no input can crash the C | audited: every `/` and `%` in `lib.c` has a provably non-zero divisor and a non-`INT_MIN` dividend on its path, so no `SIGFPE` is reachable. This is what makes exhaustive differential sweeping safe. |

## Checklist

| # | test | status |
|---|------|--------|
| 1 | `err_row01_v2_zero_any_v1` | [x] pass |
| 2 | `err_row02_v1_nonneg_v2_intmin` | [x] pass |
| 3 | `err_row03_v1_intmin_guard` | [x] pass |
| 4 | `err_row04_v1_neg_nonmin_v2_intmin` | [x] pass |
| 5 | `err_row05_both_intmin` | [x] pass |
| 6 | `err_row06_v1_intmin_v2_pos` | [x] pass |
| 7 | `err_row07_v1_intmin_v2_neg_nonmin` | [x] pass |
| 8 | `err_row08_v1_intmin_v2_minus_one_overflow` | [x] pass |
| 9 | `err_row09_tail_r_nonneg` | [x] pass |
| 10 | `err_row10_tail_r_neg_v2_pos` | [x] pass |
| 11 | `err_row11_tail_r_neg_v2_neg` | [x] pass |
| 12 | `err_row12_v1_zero` | [x] pass |
| G1–G7 | `boundary_g1_g7_generic_ffi_edges` | [x] pass |
