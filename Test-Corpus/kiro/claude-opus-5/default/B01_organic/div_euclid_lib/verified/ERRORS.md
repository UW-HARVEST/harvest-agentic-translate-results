# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c` (32 lines, the only translation
unit) and `c_src/include/lib.h`. Grepped for every rejection mechanism:

```
grep -nE 'return|assert|NULL|errno|ERROR|-1|0x7fffffff|== 0|!=|>=|<' c_src/src/lib.c
```

Findings — the complete inventory of ways this C library rejects / special-cases
input:

* error-return macros (`RETURN_ERROR`, error enums, `goto fail`): **none**
* `assert` / `abort` / `errno`: **none**
* `return NULL` / pointer parameters: **none** — the whole API is
  `int div_euclid(int, int)`, two by-value `int`s, so there is **no null-pointer
  surface and no length/size surface** to abuse
* `enum` parameters: **none** — so "out-of-range enum value across FFI" has no
  applicable parameter; the equivalent generalisation is "every one of the 2^32
  `int` bit patterns is a legal argument", which rows 1–13 and the Phase B
  randomised sweep cover, including the extremes `INT_MIN` / `INT_MAX` and
  one-step-past-boundary values (`INT_MIN+1`, `INT_MAX-1`, `-1`, `0`, `1`)
* explicit rejection: `if (v2 == 0) return 0;` — divide-by-zero guard, returns
  the sentinel `0` rather than an error code
* explicit range checks against a min constant: four separate comparisons
  against `(-0x7fffffff - 1)` (i.e. `INT_MIN`), guarding the `-v` negations that
  would otherwise be signed-overflow UB
* min/max constants present in the source: `(-0x7fffffff - 1)` = `INT_MIN`
  (appears 4×). `INT_MAX` appears only implicitly as `0x7fffffff`.

One row per distinct rejection / guard branch the C actually takes.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `div_euclid` | `v2 == 0`, `v1 > 0` (e.g. `(7, 0)`) — divide-by-zero guard at the top of the body | returns sentinel `0` (no trap, no error code) |
| 2 | `div_euclid` | `v2 == 0`, `v1 == 0` | returns `0` |
| 3 | `div_euclid` | `v2 == 0`, `v1 < 0` (e.g. `(-7, 0)`) | returns `0` |
| 4 | `div_euclid` | `v2 == 0`, `v1 == INT_MIN` — guard runs *before* the `v1 != INT_MIN` check | returns `0` |
| 5 | `div_euclid` | `v2 == 0`, `v1 == INT_MAX` | returns `0` |
| 6 | `div_euclid` | `v1 >= 0` and `v2 == INT_MIN` — 1st `v2 != (-0x7fffffff - 1)` check fails ⇒ `q = 0, r = v1` | `r = v1 >= 0` ⇒ returns `q` = `0` |
| 7 | `div_euclid` | `v1 == 0` and `v2 == INT_MIN` (row-6 boundary, `r == 0` exactly) | returns `0` |
| 8 | `div_euclid` | `v1 < 0 && v1 != INT_MIN` and `v2 == INT_MIN` — 2nd `v2 != INT_MIN` check fails ⇒ `q = 1, r = v1 - q*v2` (comma operator: `q` is already `1`) | `r = v1 - INT_MIN ∈ [1, INT_MAX] > 0` ⇒ returns `1` |
| 9 | `div_euclid` | `v1 == -1` and `v2 == INT_MIN` (row-8 extreme, `r = INT_MAX`) | returns `1` |
| 10 | `div_euclid` | `v1 == INT_MIN+1` and `v2 == INT_MIN` (row-8 other extreme, `r = 1`) | returns `1` |
| 11 | `div_euclid` | `v1 == INT_MIN` and `v2 == INT_MIN` — the `v1 != INT_MIN` check fails *and* the 3rd `v2 != INT_MIN` check fails ⇒ `q = 1, r = 0` | `r >= 0` ⇒ returns `1` |
| 12 | `div_euclid` | `v1 == INT_MIN`, `v2 >= 1` — `v1 != INT_MIN` fails, so `-v1` is never evaluated; takes the rewritten `-(v1 + v2)` path | `q = -((-(v1+v2))/v2) - 1`, `r = -((-(v1+v2)) % v2)`; e.g. `(INT_MIN, 1) -> INT_MIN`, `(INT_MIN, 2) -> -1073741824`, `(INT_MIN, 3) -> -715827883` |
| 13 | `div_euclid` | `v1 == INT_MIN`, `v2 < 0 && v2 != INT_MIN` — takes the rewritten `-(v1 - v2)` path | `q = ((-(v1-v2))/(-v2)) + 1`, `r = -((-(v1-v2)) % (-v2))`; e.g. `(INT_MIN, -2) -> 1073741824`, `(INT_MIN, INT_MIN+1) -> 2` (the `r < 0` correction fires). **`v2 == -1` makes `q = INT_MAX + 1` overflow in C** — the Rust must reproduce whatever the compiled C `.so` actually yields (measured: `INT_MIN`) |
| 14 | `div_euclid` | remainder-correction branch: `r < 0` after the main chain (only reachable when `v1 < 0, v2 > 0` or `v1 < 0, v2 < 0` with a non-zero remainder) ⇒ `return q + (v2 > 0 ? -1 : 1)` | e.g. `(-7, 2) -> -4`, `(-7, -2) -> 4`, `(INT_MIN, 3) -> -715827883` |

## Coverage of the generic C-API boundaries

| generic boundary | applicability here | covered by |
|---|---|---|
| null pointers | **N/A** — no pointer parameters | — |
| zero length | **N/A** — no length parameters; the closest analogue is the `v2 == 0` divisor guard | rows 1–5 |
| oversized length | **N/A** — no length parameters; analogue is `INT_MAX` / `INT_MIN` magnitude arguments | rows 5, 9–13 |
| one step past a valid range | `INT_MIN+1`, `INT_MAX-1`, `-1`, `0`, `1` adjacent to every guard constant | rows 7, 9, 10, and the Phase B boundary cross-product |
| out-of-range enum value across FFI | **N/A** — no `enum` parameter. Generalised: the full `int` domain is legal input; every bit pattern in the boundary set plus millions of randomised values are exercised | Phase B `tests/differential.rs` |

All 14 rows have a differential test in `translation/tests/errors.rs`.
