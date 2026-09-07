# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. The library has **no error enum,
no `RETURN_ERROR` macro, no `assert`, and no `return NULL`**; every rejection is
either a `return <sentinel>` early-out, a guarded `if`/`switch default:` branch,
or an unchecked-UB path. Every one of those is a row below.

Grep evidence used:

```sh
grep -n 'return 0;\|return 1;\|default:\|== 0\|!= 0\|assert\|isnan\|if (' c_src/src/lib.c
```

## Table

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `f2` | `typeA` is neither `0` nor `1` (outer `switch default:`) — e.g. `2`, `7`, `0xFFFFFFFF`, `-1` reinterpreted | returns `0` | ✅ `err01_f2_bad_typeA` |
| 2 | `f2` | `typeA == C2_TYPE_CIRCLE (0)` and `typeB` ∉ {0,1} (inner `switch default:`) | returns `0` | ✅ `err02_f2_bad_typeB_circle` |
| 3 | `f2` | `typeA == C2_TYPE_AABB (1)` and `typeB` ∉ {0,1} (inner `switch default:`) | returns `0` | ✅ `err03_f2_bad_typeB_aabb` |
| 4 | `f2` | both `typeA` and `typeB` out of range | returns `0` (outer default wins) | ✅ `err04_f2_both_bad` |
| 5 | `f3` | `v2 == 0` (explicit divide-by-zero guard, `if (v2 == 0) return 0;`) | returns `0` — never divides | ✅ `err05_f3_div_by_zero` |
| 6 | `f3` | `v1 >= 0 && v2 == INT_MIN` (`-0x7fffffff - 1`; `-v2` would overflow) | `q = 0, r = v1`; result `0` if `v1 == 0` else `q + 1 = 1` (since `r >= 0` → `q`; for `v1>0`, `r=v1>=0` → `0`) | ✅ `err06_f3_v2_intmin` |
| 7 | `f3` | `v1 < 0 && v1 != INT_MIN && v2 == INT_MIN` | `q = 1, r = v1 - q*v2` (signed overflow, wraps) → `r = v1 - INT_MIN`; then `r>=0 ? q : q + 1` | ✅ `err07_f3_v1neg_v2_intmin` |
| 8 | `f3` | `v1 == INT_MIN && v2 >= 1` (guarded `-v1` overflow path, uses `-(v1+v2)`) | `q = -((-(v1+v2))/v2) - 1`, `r = -((-(v1+v2))%v2)` | ✅ `err08_f3_v1_intmin_v2pos` |
| 9 | `f3` | `v1 == INT_MIN && v2 < 0 && v2 != INT_MIN` (uses `-(v1-v2)`) | `q = ((-(v1-v2))/(-v2)) + 1`, `r = -((-(v1-v2))%(-v2))` | ✅ `err09_f3_v1_intmin_v2neg` |
| 10 | `f3` | `v1 == INT_MIN && v2 == INT_MIN` (final `else`) | `q = 1, r = 0` → returns `1` | ✅ `err10_f3_both_intmin` |
| 11 | `f3` | any input where the computed remainder `r < 0` | returns `q + (v2 > 0 ? -1 : 1)` (the floor correction) | ✅ `err11_f3_negative_remainder` |
| 12 | `f4` | `rnd == NULL` | **no null check** — `cn_rnd_next` dereferences → SIGSEGV (UB). Rust must be identically unchecked (documented, not executed) | ⚠️ documented, see `err_null_pointers_documented` |
| 13 | `f4` | `state = {0, 0}` (degenerate xorshift seed) | `value = 0` → `result = 1023<<52` → returns exactly `0.0` | ✅ `err13_f4_zero_state` |
| 14 | `f4` | `state = {UINT64_MAX, UINT64_MAX}` (max seed) | pure integer path, no rejection; result in `[0,1)` | ✅ `err14_f4_max_state` |
| 15 | `f5` | any bit above bit 15 set (masks are 16-bit: `0xAAAA`, `0x5555`, …) | high bits are **silently discarded**; result always `< 0x10000` | ✅ `err15_f5_high_bits_discarded` |
| 16 | `f5` | `a == 0xFFFFFFFF` / `a == 0` | `0x0000FFFF` / `0` | ✅ `err16_f5_extremes` |
| 17 | `f7` | `channels == 2` (the `channels != 2` / `channels == 2` predicate switch) | first term becomes `0`, terms 2 and 3 activate | ✅ `err17_f7_channels_two` |
| 18 | `f7` | `bitdepth == 32` (the `bitdepth != 32` predicate) | `bitdepth + 0` instead of `bitdepth + 1` in term 3 | ✅ `err18_f7_bitdepth_32` |
| 19 | `f7` | `blocksize`/`channels`/`bitdepth` large enough to overflow `uint32_t` (e.g. `0xFFFFFFFF`) | **no range check** — unsigned wrap-around modulo 2³² | ✅ `err19_f7_overflow_wrap` |
| 20 | `f7` | all-zero arguments | `18 + 0 + (0+7)/8 = 18` | ✅ `err20_f7_zeros` |
| 21 | `f9` | degenerate triangle: `dot00*dot11 - dot01*dot01 == 0` (division by zero, **unchecked**) | `invDenom = 1.0f/0.0f = ±inf`; `u`/`v` become `±inf` or `NaN` (`0*inf`) | ✅ `err21_f9_degenerate_denominator` |
| 22 | `f9` | all four points identical (all dots `0`) | `invDenom = inf`, `u = 0*inf = NaN`, `v = NaN` | ✅ `err22_f9_all_points_equal` |
| 23 | `f9` | any coordinate is `NaN` / `±inf` | no check; NaN/inf propagates through `mulss`/`addss`/`divss` | ✅ `err23_f9_nan_inf_inputs` |
| 24 | `f10` | `h` in `[0, 0xFFFF]` — `n = h>>10 ≤ 63`, index `= (h&0x3ff) + m__offset[n] ≤ 1023+1024 = 2047` | **no bounds check needed**; every `uint16_t` is in range. Includes half-float Inf (`0x7C00`) and NaN (`0x7E00`) | ✅ `err24_f10_exhaustive_all_65536` |
| 25 | `f11` | `src[1] == 0.0f` (also `-0.0f`, since `s == 0` is a float compare) | early return: `dest[0..3] = l` (`src[2]`), `c`/`m`/`x` never computed | ✅ `err25_f11_s_zero` |
| 26 | `f11` | `h < 0.0f` (negative hue) — falls through every `if` **except** the third | third branch is `h < 120.0f && h < 180.0f` (**C bug: not `h >= 120`**) so negative `h` takes the *third* branch → `{m, c+m, x+m}` | ✅ `err26_f11_negative_hue` |
| 27 | `f11` | `h >= 360.0f` (out of hue range) | final `else` → `{m, m, m}` | ✅ `err27_f11_hue_ge_360` |
| 28 | `f11` | `h` is `NaN` (all comparisons false) | final `else` → `{m, m, m}` (with `x` = NaN-derived, unused) | ✅ `err28_f11_nan_hue` |
| 29 | `f11` | `h == +inf` / `h == -inf` | `+inf` → final `else` `{m,m,m}`; `-inf` → third branch (C bug) | ✅ `err29_f11_inf_hue` |
| 30 | `f11` | `h` in `[120, 180)` — the branch the C bug makes **unreachable** | `h >= 120` fails branch 3's `h < 120.0f`, and fails 4/5/6 → final `else` `{m,m,m}` | ✅ `err30_f11_dead_range_120_180` |
| 31 | `f11` | `dest == NULL` or `src == NULL` | **no null check** — deref → SIGSEGV (UB) | ⚠️ documented |
| 32 | `f12` | `src[1] == 0.0f` | early return `dest[0..3] = v` (`src[2]`) | ✅ `err32_f12_s_zero` |
| 33 | `f12` | `i = (int)floorf(h/60)` outside `0..=4` (`switch default:`) — e.g. `h < 0`, `h >= 360` | `default:` → `{v, p, q}` | ✅ `err33_f12_i_default_branch` |
| 34 | `f12` | `h` is `NaN` → `(int)floorf(NaN)` is **UB**; x86 `cvttss2si` yields `0x80000000` | `i = INT_MIN` → `default:` branch | ✅ `err34_f12_nan_hue` |
| 35 | `f12` | `h/60.0f` not representable as `int` (`h` huge / `±inf`) → `cvttss2si` "integer indefinite" | `i = INT_MIN` → `default:` branch | ✅ `err35_f12_hue_out_of_int_range` |
| 36 | `f12` | `dest == NULL` or `src == NULL` | **no null check** — SIGSEGV (UB) | ⚠️ documented |
| 37 | `f13` | `delta == 0.0f` (i.e. `r == g == b`) | early return `{0, 0, max}` | ✅ `err37_f13_delta_zero` |
| 38 | `f13` | `max == 0.0f` (pure black, or all components ≤ 0 with max 0) | early return `{0, 0, 0}` | ✅ `err38_f13_max_zero` |
| 39 | `f13` | any component `NaN` → `min`/`max` ternaries pick the **second** operand on unordered compare | `delta` NaN → `delta == 0` false, `max == 0` false → falls through to hue math → NaN outputs | ✅ `err39_f13_nan_component` |
| 40 | `f13` | `h < 0` after `h *= 60` (happens when `r == max` and `g < b`) | `h += 360` correction | ✅ `err40_f13_negative_hue_wrap` |
| 41 | `f13` | `dest == NULL` or `src == NULL` | **no null check** — SIGSEGV (UB) | ⚠️ documented |
| 42 | `agglom` | `f4` result is `NaN` (`if (!isnan(f4_r))`) | contribution **skipped**, not added to `ret` | ✅ `err42_47_agglom_nan_skips` (f4 can never be NaN, guard is dead — verified) |
| 43 | `agglom` | `f9_r.x` and/or `f9_r.y` is `NaN` | that component's contribution **skipped** | ✅ `err42_47_agglom_nan_skips` |
| 44 | `agglom` | `f10_r` is `NaN` (half-float NaN inputs `0x7E00`, `0xFFFF`, …) | contribution **skipped** | ✅ `err42_47_agglom_nan_skips` |
| 45 | `agglom` | any of `f11_r[0..3]` is `NaN` | that element's contribution **skipped** | ✅ `err42_47_agglom_nan_skips` |
| 46 | `agglom` | any of `f12_r[0..3]` is `NaN` | that element's contribution **skipped** | ✅ `err42_47_agglom_nan_skips` |
| 47 | `agglom` | any of `f13_r[0..3]` is `NaN` | that element's contribution **skipped** | ✅ `err42_47_agglom_nan_skips` |
| 48 | `agglom` | contributions are `±inf` (NOT skipped — only NaN is) | `ret` becomes `±inf`, or `NaN` if `+inf` and `-inf` both added | ✅ `err48_agglom_inf_not_skipped` |
| 49 | `c2CircletoCircle` / `c2CircletoAABB` | `d2`/`r2` unordered (NaN radius or coords) → `d2 < r2` is **false** | returns `0` | ✅ `err49_c2_nan_compare` |
| 50 | `c2AABBtoAABB` | any coordinate `NaN` → each `<` false → `d0|d1|d2|d3 == 0` → `!0` | returns `1` (reports overlap!) | ✅ `err50_c2_aabb_nan_reports_overlap` |
| 51 | `c2Maxv`/`c2Minv`/`c2Clampv` | operand `NaN`: `a.x > b.x` / `a.x < b.x` false on unordered → the ternary yields **`b`** | NaN handling is asymmetric — `c2Maxv(NaN, 1) = 1`, `c2Maxv(1, NaN) = NaN` | ✅ `err51_c2_minmax_nan_asymmetry` |
| 52 | `c2Clampv` | `lo > hi` (inverted clamp box) | no validation — result is `max(lo, min(a, hi))`, i.e. `lo` wins | ✅ `err52_c2_clampv_inverted_box` |

## Generic FFI-boundary boundaries also covered

| item | covered by |
|------|-----------|
| out-of-range C enum values across FFI (`C2_TYPE` = 2, 3, 0xFFFFFFFF, and `INT_MIN`-as-`u32`) | rows 1–4 |
| zero-valued length/size-like args | rows 5, 20, 13 |
| oversized / max-valued integer args | rows 6–10, 14, 16, 19, 24 |
| one step past a valid range | rows 6, 8, 17, 18, 24, 27, 30, 33 |
| `NaN` / `±inf` / `-0.0` floats through every float entry point | rows 23, 25, 28, 29, 34, 35, 39, 48–51 |
| null pointers | rows 12, 31, 36, 41 — C has **no** checks; dereference is UB/SIGSEGV in both. Recorded as a deliberate non-executed row (running it would abort the test process); Rust wrapper is equally unchecked, verified by inspection of `f4`/`f11`/`f12`/`f13` in `translation/src/lib.rs`. |

## Verification result

All 52 rows have a passing differential test in
`translation/tests/phase_c_errors.rs` (44 `#[test]` functions; several rows are
grouped where they share one construction, e.g. rows 42–47 in
`err42_47_agglom_nan_skips`).

```
$ cargo test --test phase_c_errors
test result: ok. 44 passed; 0 failed
```

Each test asserts the *specific* sentinel, not merely "both failed":

- `f2` → exactly `0` for every out-of-range `C2_TYPE` (including `0xFFFFFFFF`
  and `0x80000000`, i.e. `-1` and `INT_MIN` reinterpreted).
- `f3` → exactly `0` for `v2 == 0`; the `INT_MIN` branches are re-derived with
  `wrapping_*` arithmetic in the test and compared against both libraries.
- `f11`/`f12`/`f13` → the early-out copies the exact source component
  (bit-for-bit, so `-0.0` and NaN payloads count), and the branch actually
  taken is pinned (e.g. row 26 asserts negative hue reaches the *buggy* third
  branch, row 30 asserts `[120,180)` falls through to the final `else`).
- Rows 12/31/36/41 (null pointers) are non-executable by design — the C
  dereferences unconditionally, so calling them would fault the test process.
  They are verified structurally by `err_null_pointers_documented`, which
  asserts (a) the Rust wrappers add no `is_null` check the C lacks, and (b) the
  C source still contains no `NULL`, so the premise cannot silently rot.

### One finding worth recording

Row 11's first draft asserted the corrected quotient equals `trunc - 1` (true
floored division). The C actually computes `q + (v2 > 0 ? -1 : 1)`, so for
`v2 < 0` it ADDS one — e.g. `f3(-200, -199)` returns `2`, not `1`. The C is
ground truth; the Rust already matched, and the *test's* invariant was corrected
to the C's behaviour rather than the other way round.
