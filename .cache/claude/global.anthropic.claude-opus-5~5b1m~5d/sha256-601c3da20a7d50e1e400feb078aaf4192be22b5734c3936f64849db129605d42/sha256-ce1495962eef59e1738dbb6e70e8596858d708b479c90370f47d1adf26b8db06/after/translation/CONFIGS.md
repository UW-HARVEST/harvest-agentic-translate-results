# CONFIGS.md — Configuration / valid-input surface table (Phase A)

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from the
branches `c_src/src/lib.c` actually takes.

## Axis enumeration (mechanical)

### Runtime options / modes / flags

`grep -n '#if\|#ifdef\|switch\|extern\|static\|struct\|typedef' c_src/src/lib.c`
→ **no matches**. There is:

* no global/`static` state,
* no init/config/setter function,
* no `#ifdef` compile-time mode,
* no `switch`,
* no struct/handle/context.

`include/lib.h` declares exactly one entry point. So the "options" axis is
**empty**: the only configuration is the *input shape* of the two `int`
arguments. `div_euclid` is simultaneously the highest-level and the
**lowest-level** public entry point — there is no convenience wrapper hiding a
deeper API, so "exercise the low-level entry points directly" is satisfied by
calling `div_euclid` itself through both `.so`s.

### Input-shape axes the C branches on

Axis **V1** — class of `v1`, from `if (v1 >= 0)` (line 8) and
`v1 != (-0x7fffffff - 1)` (line 15):

| id | class | boundary representatives |
|----|-------|--------------------------|
| `P1` | `v1 > 0` | `1`, `2`, `INT_MAX-1`, `INT_MAX`, random |
| `Z1` | `v1 == 0` | `0` |
| `N1` | `INT_MIN < v1 < 0` | `-1`, `-2`, `INT_MIN+1`, `INT_MIN+2`, random |
| `M1` | `v1 == INT_MIN` | `-2147483648` |

Axis **V2** — class of `v2`, from `if (v2 == 0)` (line 4), `if (v2 >= 0)`
(lines 9, 16, 22) and `v2 != (-0x7fffffff - 1)` (lines 11, 18, 24):

| id | class | boundary representatives |
|----|-------|--------------------------|
| `P2` | `v2 > 0` | `1`, `2`, `3`, `INT_MAX-1`, `INT_MAX`, random |
| `Z2` | `v2 == 0` | `0` (→ `ERRORS.md` row 1) |
| `N2` | `INT_MIN < v2 < 0` | `-1`, `-2`, `-3`, `INT_MIN+1`, random |
| `M2` | `v2 == INT_MIN` | `-2147483648` |

Axis **R** — divisibility, which selects the tail branch at line 28
(`if (r >= 0)`); this is a *data* axis, not an option, and it is the one most
easily missed:

| id | shape | how the test constructs it |
|----|-------|----------------------------|
| `EX` | exact multiple (`r == 0`) | pick `k`, `v2`, set `v1 = k * v2` (wrapping) |
| `NX` | not a multiple (`r != 0`) | pick `v1` such that `v1 % v2 != 0` |

Axis **MAG** — magnitude relation, distinguishing quotient `0` from non-zero
and single-digit from full-width quotients:

| id | shape |
|----|-------|
| `SM` | `abs(v1) < abs(v2)` → quotient magnitude 0 |
| `EQ` | `abs(v1) == abs(v2)` → quotient magnitude 1 |
| `LG` | `abs(v1) > abs(v2)` → quotient magnitude > 1, incl. `v2 == ±1` full-width |

## The 9 leaf branches of the C `if`/`else` ladder

Establishing the dangling-`else` parse first, because the whole table depends on
it. C binds each `else` to the nearest unmatched `if`, so the ladder is:

```
if (v1 >= 0)              { if (v2>=0) L1 else if (v2!=MIN) L2 else L3 }
else if (v1 != MIN)       { if (v2>=0) L4 else if (v2!=MIN) L5 else L6 }
else if (v2 >= 0)         L7
else if (v2 != MIN)       L8
else                      L9
```

which matches the source indentation.

| leaf | line | condition | body | reachable tail branches |
|------|------|-----------|------|--------------------------|
| L1 | 10 | `v1>=0, v2>0` | `return v1/v2` | **none** — returns early, skips line 28 |
| L2 | 12 | `v1>=0, v2<0, v2!=MIN` | `q=-(v1/-v2), r=v1%(-v2)` | `r >= 0` only (`v1>=0`, `-v2>0`) |
| L3 | 14 | `v1>=0, v2==MIN` | `q=0, r=v1` | `r >= 0` only |
| L4 | 17 | `v1<0,v1!=MIN, v2>0` | `q=-((-v1)/v2), r=-((-v1)%v2)` | `r>=0` if `EX`, `r<0` if `NX` (→ `q-1`) |
| L5 | 19 | `v1<0,v1!=MIN, v2<0,v2!=MIN` | `q=(-v1)/(-v2), r=-((-v1)%(-v2))` | `r>=0` if `EX`, `r<0` if `NX` (→ `q+1`) |
| L6 | 21 | `v1<0,v1!=MIN, v2==MIN` | `q=1, r=v1-q*v2` | `r >= 0` only |
| L7 | 23 | `v1==MIN, v2>0` | `q=-((-(v1+v2))/v2)-1, r=-((-(v1+v2))%v2)` | `r>=0` if `EX`, `r<0` if `NX` (→ `q-1`) |
| L8 | 25 | `v1==MIN, v2<0, v2!=MIN` | `q=((-(v1-v2))/(-v2))+1, r=-((-(v1-v2))%(-v2))` | `r>=0` if `EX`, `r<0` if `NX` (→ `q+1`) |
| L9 | 27 | `v1==MIN, v2==MIN` | `q=1, r=0` | `r >= 0` only |

Note the asymmetry the table makes visible and which a translation can easily
get wrong: **L2's `r` is *not* negated** (`r = v1 % (-v2)`) while L5's is
(`r = -((-v1) % (-v2))`).

## Configuration table

Cross-product of the axes, pruned to combinations the C treats differently.
Every row is driven with **many randomized inputs** (fixed seed) plus its
boundary representatives.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `div_euclid` | L1 · `P1`×`P2` · `EX` · `LG` — positive ÷ positive, exact multiple (e.g. `84/12`) | [x] |
| 2 | `div_euclid` | L1 · `P1`×`P2` · `NX` · `LG` — positive ÷ positive, remainder (e.g. `85/12`) | [x] |
| 3 | `div_euclid` | L1 · `P1`×`P2` · `SM` — `0 < v1 < v2`, quotient 0 | [x] |
| 4 | `div_euclid` | L1 · `P1`×`P2` · `EQ` — `v1 == v2`, quotient 1 | [x] |
| 5 | `div_euclid` | L1 · `Z1`×`P2` — `v1 == 0`, `v2 > 0` | [x] |
| 6 | `div_euclid` | L1 · `P1`×`P2` with `v2 == 1` — full-width quotient, incl. `v1 == INT_MAX` | [x] |
| 7 | `div_euclid` | L1 · `P1`×`P2` with `v2 == INT_MAX` and `v1 == INT_MAX` / `INT_MAX-1` | [x] |
| 8 | `div_euclid` | L2 · `P1`×`N2` · `EX` · `LG` — positive ÷ negative, exact (e.g. `84/-12`) | [x] |
| 9 | `div_euclid` | L2 · `P1`×`N2` · `NX` · `LG` — positive ÷ negative, remainder → tests the **non-negated `r`** | [x] |
| 10 | `div_euclid` | L2 · `P1`×`N2` · `SM` — `0 < v1 < -v2`, quotient 0 | [x] |
| 11 | `div_euclid` | L2 · `P1`×`N2` · `EQ` — `v1 == -v2` | [x] |
| 12 | `div_euclid` | L2 · `Z1`×`N2` — `v1 == 0`, `v2 < 0` (`q = -0`) | [x] |
| 13 | `div_euclid` | L2 · `P1`×`N2` with `v2 == -1` (`v1` up to `INT_MAX`) and with `v2 == INT_MIN+1` | [x] |
| 14 | `div_euclid` | L3 · `P1`/`Z1` × `M2` — `v1 >= 0`, `v2 == INT_MIN` (range-check fallback) | [x] |
| 15 | `div_euclid` | L4 · `N1`×`P2` · `EX` · `LG` — negative ÷ positive, exact (e.g. `-84/12`) | [x] |
| 16 | `div_euclid` | L4 · `N1`×`P2` · `NX` · `LG` — negative ÷ positive, remainder → tail `q-1` | [x] |
| 17 | `div_euclid` | L4 · `N1`×`P2` · `SM` — `-v2 < v1 < 0`, quotient 0 then `-1` | [x] |
| 18 | `div_euclid` | L4 · `N1`×`P2` · `EQ` — `v1 == -v2` | [x] |
| 19 | `div_euclid` | L4 · `N1`×`P2` with `v2 == 1`, `v1 == INT_MIN+1` — full-width | [x] |
| 20 | `div_euclid` | L4 · `N1`×`P2` with `v2 == INT_MAX`, `v1 == INT_MIN+1` | [x] |
| 21 | `div_euclid` | L5 · `N1`×`N2` · `EX` · `LG` — negative ÷ negative, exact | [x] |
| 22 | `div_euclid` | L5 · `N1`×`N2` · `NX` · `LG` — negative ÷ negative, remainder → tail `q+1` | [x] |
| 23 | `div_euclid` | L5 · `N1`×`N2` · `SM` — quotient 0 then `+1` | [x] |
| 24 | `div_euclid` | L5 · `N1`×`N2` · `EQ` — `v1 == v2` | [x] |
| 25 | `div_euclid` | L5 · `N1`×`N2` with `v2 == -1` and `v1 == INT_MIN+1` — full-width negation | [x] |
| 26 | `div_euclid` | L5 · `N1`×`N2` with `v2 == INT_MIN+1` (`abs` = `INT_MAX`) | [x] |
| 27 | `div_euclid` | L6 · `N1`×`M2` — `INT_MIN < v1 < 0`, `v2 == INT_MIN` (range-check fallback, `r = v1 - INT_MIN`) | [x] |
| 28 | `div_euclid` | L7 · `M1`×`P2` · `EX` — `v1 == INT_MIN`, `v2` a positive power of two / divisor of `2^31` (`1,2,4,…,2^31` n/a → up to `2^30`, and `INT_MIN` itself excluded) | [x] |
| 29 | `div_euclid` | L7 · `M1`×`P2` · `NX` — `v1 == INT_MIN`, `v2 > 0` non-divisor → tail `q-1` | [x] |
| 30 | `div_euclid` | L7 · `M1`×`P2` with `v2 == 1` — the `-(v1+v2)` rewrite at its extreme | [x] |
| 31 | `div_euclid` | L7 · `M1`×`P2` with `v2 == INT_MAX` and `v2 == INT_MAX-1` | [x] |
| 32 | `div_euclid` | L8 · `M1`×`N2` · `EX` — `v1 == INT_MIN`, `v2` a negative divisor of `2^31` | [x] |
| 33 | `div_euclid` | L8 · `M1`×`N2` · `NX` — `v1 == INT_MIN`, `v2 < 0` non-divisor → tail `q+1` | [x] |
| 34 | `div_euclid` | L8 · `M1`×`N2` with `v2 == -1` — **signed-overflow** `q = INT_MAX + 1` (also `ERRORS.md` row 8) | [x] |
| 35 | `div_euclid` | L8 · `M1`×`N2` with `v2 == INT_MIN+1` | [x] |
| 36 | `div_euclid` | L9 · `M1`×`M2` — both `INT_MIN` | [x] |
| 37 | `div_euclid` | `Z2` — `v2 == 0` across all `v1` classes (valid input, defined result `0`) | [x] |
| 38 | `div_euclid` | full 2-D exhaustive sweep of `[-512, 512]^2` — every leaf × every tail × every divisibility at small magnitudes | [x] |
| 39 | `div_euclid` | boundary-neighbourhood cross-product: `{INT_MIN..INT_MIN+8} ∪ {-8..8} ∪ {INT_MAX-8..INT_MAX}` squared | [x] |
| 40 | `div_euclid` | uniform random `(i32, i32)` over the **entire** 2×32-bit domain, seeded PCG, 4,000,000 pairs | [x] |
| 41 | `div_euclid` | single-axis full sweeps: `v1` pinned to each boundary rep while `v2` walks a dense stride over all `2^32`, and vice-versa | [x] |
| 42 | `div_euclid` | power-of-two and `±(2^k ± 1)` structured operands for both arguments (all 32 `k`, all sign combos) | [x] |

| 43 | `div_euclid` | **meta-row**: assert the boundary cross-product provably reaches all ten control paths (`v2==0` early return + L1..L9), so no leaf can be silently unexercised | [x] |

## Checklist status

All 43 rows are exercised by `translation/tests/phase_b_configs.rs` (test names
`cfg_row01_*` .. `cfg_row42_*` plus `cfg_meta_all_ten_control_paths_reached`);
the row -> test mapping is in that file's header comment. Every row was checked
off only after it passed across its randomized inputs against the C `.so`.

**Verified result: 43/43 rows pass**, under all 6 configurations
(dev/release x default/`--no-default-features`/`--all-features`).

### Leaf-coverage evidence

`leaf_of(v1, v2)` in the test file re-implements the C ladder's branch selection
independently, and:

* `cfg_meta_all_ten_control_paths_reached` asserts all ten paths are reached;
* `cfg_row40_uniform_random_full_domain` asserts it ran all 4,000,000 iterations
  *and* that each of the four bulk leaves (L1, L2, L4, L5) was hit > 500,000
  times, so "4 million random pairs" cannot degenerate into one code path;
* `cfg_row38_exhaustive_small_square` asserts exactly 1,050,625 comparisons;
* `cfg_row41_single_axis_full_sweeps` asserts > 2,000,000 comparisons.
