# CONFIGS.md — configuration surface table (valid inputs)

Derived mechanically from the branch structure of `c_src/src/lib.c`, not from
assumptions about which cases "matter".

## Public entry points (complete set)

`c_src/include/lib.h` declares exactly one prototype and `src/lib.c` defines
exactly one function:

* `int div_euclid(int v1, int v2)` — this is simultaneously the lowest-level and
  the only entry point. There are no convenience wrappers, no init/teardown
  functions, no context object, and no state to set up, so "drive it the way a
  real consumer does" reduces to calling it directly across its whole input
  domain. Both `.so`s are exercised through `dlopen` + `dlsym` (`libloading`),
  never by direct Rust calls.

## Runtime options / modes / flags

```
grep -nE '#if|#ifdef|#ifndef|switch|extern|static|global' c_src/src/lib.c c_src/include/lib.h
```

→ **no matches.** There is no `#ifdef`, no `switch`, no global/`static` state, no
setter, and no flag parameter. The library is a pure function of its two
arguments, so the configuration surface is composed **entirely of input shape**.
Likewise `translation/Cargo.toml` has no `[features]` table, so there is exactly
one build configuration.

## Axes the C actually branches on

Read off the `if`/`else` chain (dangling-`else` binding as written):

* **A1 — class of `v1`** (4 values): `v1 > 0` · `v1 == 0` · `INT_MIN < v1 < 0` ·
  `v1 == INT_MIN` (`v1 >= 0` test, then `v1 != (-0x7fffffff - 1)` test)
* **A2 — class of `v2`** (4 values): `v2 == 0` · `v2 > 0` · `INT_MIN < v2 < 0` ·
  `v2 == INT_MIN` (`v2 == 0` guard, `v2 >= 0` test, `v2 != (-0x7fffffff - 1)`
  test — this last one appears 3× on 3 different paths)
* **A3 — remainder shape**, which selects the trailing `if (r >= 0)` correction:
  `r == 0` (exact division) · `r != 0` (correction `q + (v2 > 0 ? -1 : 1)` fires
  when `r < 0`)
* **A4 — relative magnitude**: `|v1| < |v2|` (quotient 0) · `|v1| == |v2|` ·
  `|v1| > |v2|`
* **A5 — magnitude extremes**, which is where the guarded negations and the
  `q ± 1` adjustments can overflow: `|v2| == 1` · `v1 ∈ {INT_MAX, INT_MIN,
  INT_MIN+1}` · `v2 ∈ {INT_MAX, INT_MIN, INT_MIN+1, -1, 1}`

Rows below are the cross-product of A1 × A2, refined by A3/A4/A5 wherever the C
takes a different path or a different arithmetic expression. `v2 == 0` rows live
in `ERRORS.md` (rows 1–5) since that is a rejection, not a valid configuration.

Every row is driven with **many randomized inputs (fixed seed `0x2545F491_4F6CDD1D`,
xorshift64\*)** drawn from that row's class — not one hand-picked value — and both
`.so`s must agree byte-for-byte on every draw. Rows that pin all bits (e.g. row
46) are exact singletons and are asserted once.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| | | **Group A — `v1 >= 0`, `v2 > 0` → early `return v1 / v2`** | |
| 1 | `div_euclid` | `v1 == 0`, `v2` random in `[1, INT_MAX]` | [x] |
| 2 | `div_euclid` | `0 < v1 < v2`, both random (quotient 0, `r != 0`) | [x] |
| 3 | `div_euclid` | `v1 == v2`, random in `[1, INT_MAX]` (quotient 1, `r == 0`) | [x] |
| 4 | `div_euclid` | `v1 > v2 > 0`, `v1` an exact multiple of `v2` (`r == 0`) | [x] |
| 5 | `div_euclid` | `v1 > v2 > 0`, non-exact (`r != 0`) | [x] |
| 6 | `div_euclid` | `v2 == 1`, `v1` random in `[0, INT_MAX]` (identity) | [x] |
| 7 | `div_euclid` | `v1 == INT_MAX`, `v2 == 1` (max magnitude, `|v2|` minimal) | [x] |
| 8 | `div_euclid` | `v1 == INT_MAX`, `v2 == INT_MAX` (equal extremes) | [x] |
| 9 | `div_euclid` | `v1 == INT_MAX`, `v2 == 2` (max magnitude, odd numerator) | [x] |
| 10 | `div_euclid` | `v1` random in `[0, INT_MAX)`, `v2 == INT_MAX` (`|v1| < |v2|`) | [x] |
| | | **Group B — `v1 >= 0`, `INT_MIN < v2 < 0` → `q = -(v1/-v2)`, `r = v1 % -v2` (always `r >= 0`)** | |
| 11 | `div_euclid` | `v1 == 0`, `v2` random in `[INT_MIN+1, -1]` | [x] |
| 12 | `div_euclid` | `0 < v1 < -v2`, both random (quotient 0) | [x] |
| 13 | `div_euclid` | `v1 == -v2`, random (`r == 0`) | [x] |
| 14 | `div_euclid` | `v1 > -v2`, exact multiple (`r == 0`) | [x] |
| 15 | `div_euclid` | `v1 > -v2`, non-exact (`r > 0`) | [x] |
| 16 | `div_euclid` | `v2 == -1`, `v1` random in `[0, INT_MAX]` (negation extreme) | [x] |
| 17 | `div_euclid` | `v1 == INT_MAX`, `v2 == -1` | [x] |
| 18 | `div_euclid` | `v1 == INT_MAX`, `v2 == INT_MIN+1` (`-v2 == INT_MAX`) | [x] |
| | | **Group C — `v1 >= 0`, `v2 == INT_MIN` → `q = 0, r = v1`** | |
| 19 | `div_euclid` | `v1` random in `[0, INT_MAX]`, `v2 == INT_MIN`, incl. `v1 ∈ {0, 1, INT_MAX}` | [x] |
| | | **Group D — `INT_MIN < v1 < 0`, `v2 > 0` → `q = -(-v1/v2)`, `r = -(-v1 % v2)`; correction `q-1` when `r < 0`** | |
| 20 | `div_euclid` | `-v1 < v2`, both random (quotient 0, `r < 0` ⇒ result `-1`) | [x] |
| 21 | `div_euclid` | `-v1 == v2`, random (`r == 0`, no correction) | [x] |
| 22 | `div_euclid` | `-v1 > v2`, exact multiple (`r == 0`, no correction) | [x] |
| 23 | `div_euclid` | `-v1 > v2`, non-exact (`r < 0` ⇒ correction fires) | [x] |
| 24 | `div_euclid` | `v2 == 1`, `v1` random in `[INT_MIN+1, -1]` (`r == 0` always) | [x] |
| 25 | `div_euclid` | `v1 == INT_MIN+1`, `v2 == 1` (`-v1 == INT_MAX`) | [x] |
| 26 | `div_euclid` | `v1 == INT_MIN+1`, `v2 == INT_MAX` (`-v1 == v2`, exact) | [x] |
| 27 | `div_euclid` | `v1 == -1`, `v2 == INT_MAX` (min magnitude vs max, correction) | [x] |
| | | **Group E — `INT_MIN < v1 < 0`, `INT_MIN < v2 < 0` → `q = -v1/-v2`, `r = -(-v1 % -v2)`; correction `q+1` when `r < 0`** | |
| 28 | `div_euclid` | `-v1 < -v2`, both random (quotient 0, correction ⇒ `1`) | [x] |
| 29 | `div_euclid` | `-v1 == -v2`, random (`r == 0`) | [x] |
| 30 | `div_euclid` | `-v1 > -v2`, exact multiple (`r == 0`) | [x] |
| 31 | `div_euclid` | `-v1 > -v2`, non-exact (correction fires) | [x] |
| 32 | `div_euclid` | `v2 == -1`, `v1` random in `[INT_MIN+1, -1]` | [x] |
| 33 | `div_euclid` | `v1 == INT_MIN+1`, `v2 == -1` (both negations at the extreme) | [x] |
| 34 | `div_euclid` | `v1 == -1`, `v2 == INT_MIN+1` | [x] |
| | | **Group F — `INT_MIN < v1 < 0`, `v2 == INT_MIN` → `q = 1, r = v1 - q*v2` (comma-operator sequencing)** | |
| 35 | `div_euclid` | `v1` random in `[INT_MIN+1, -1]`, `v2 == INT_MIN`, incl. `v1 ∈ {-1, INT_MIN+1}` | [x] |
| | | **Group G — `v1 == INT_MIN`, `v2 > 0` → rewritten `-(v1 + v2)` path, `q = -((-(v1+v2))/v2) - 1`** | |
| 36 | `div_euclid` | `v1 == INT_MIN`, `v2 == 1` (`q` lands exactly on `INT_MIN`) | [x] |
| 37 | `div_euclid` | `v1 == INT_MIN`, `v2 == 2` (exact, `r == 0`) | [x] |
| 38 | `div_euclid` | `v1 == INT_MIN`, `v2 == 3` (non-exact ⇒ `r < 0` correction) | [x] |
| 39 | `div_euclid` | `v1 == INT_MIN`, `v2 == INT_MAX` (`v1 + v2 == -1`) | [x] |
| 40 | `div_euclid` | `v1 == INT_MIN`, `v2` random in `[1, INT_MAX]`, plus every `v2 ∈ [1, 4096]` | [x] |
| | | **Group H — `v1 == INT_MIN`, `INT_MIN < v2 < 0` → rewritten `-(v1 - v2)` path, `q = ((-(v1-v2))/(-v2)) + 1`** | |
| 41 | `div_euclid` | `v1 == INT_MIN`, `v2 == -1` (**`q = INT_MAX + 1` overflows in C**; Rust must match the compiled C `.so`) | [x] |
| 42 | `div_euclid` | `v1 == INT_MIN`, `v2 == -2` (exact) | [x] |
| 43 | `div_euclid` | `v1 == INT_MIN`, `v2 == -3` (non-exact ⇒ correction) | [x] |
| 44 | `div_euclid` | `v1 == INT_MIN`, `v2 == INT_MIN+1` (`-v2 == INT_MAX`) | [x] |
| 45 | `div_euclid` | `v1 == INT_MIN`, `v2` random in `[INT_MIN+1, -1]`, plus every `v2 ∈ [-4096, -1]` | [x] |
| | | **Group I — both at the guarded minimum** | |
| 46 | `div_euclid` | `v1 == INT_MIN`, `v2 == INT_MIN` → `q = 1, r = 0` (exact singleton) | [x] |
| | | **Group J — saturation sweeps over the whole domain (all classes mixed)** | |
| 47 | `div_euclid` | full cross-product of the 76-value boundary set × itself (5 776 pairs: `0, ±1, ±2, ±3, ±5, ±7, ±10, ±100, ±255, ±256, ±257, ±65535, ±65536, ±65537, ±2^24, ±(2^30±1), ±2^30, ±(2^31-2), INT_MAX, INT_MIN, INT_MIN+1, INT_MIN+2, …`) | [x] |
| 48 | `div_euclid` | exhaustive `v1, v2 ∈ [-400, 400]` (641 601 pairs — every sign/magnitude/remainder combination in the small band) | [x] |
| 49 | `div_euclid` | 3 000 000 uniformly random full-range `(v1, v2)` pairs, fixed seed | [x] |
| 50 | `div_euclid` | 400 000 random pairs where one side is drawn from the boundary set and the other is full-range random (both orders) — mixes extremes with arbitrary values | [x] |
| 51 | `div_euclid` | 400 000 random pairs with small `|v2| ∈ [1, 64]` and full-range `v1` (dense remainder coverage at every extreme numerator) | [x] |
| 52 | `div_euclid` | 400 000 random *exact-multiple* pairs (`v1 = k * v2`, wrapping) across all four sign combinations (`r == 0` at scale) | [x] |
