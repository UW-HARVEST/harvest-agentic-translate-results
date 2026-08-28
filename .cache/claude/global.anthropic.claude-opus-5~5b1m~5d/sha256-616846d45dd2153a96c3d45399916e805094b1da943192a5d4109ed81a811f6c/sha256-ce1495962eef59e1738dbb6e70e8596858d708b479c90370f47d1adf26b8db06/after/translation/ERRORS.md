# ERRORS.md — error / rejection surface table (Phase A → Phase C)

Derived **mechanically** from `c_src/src/lib.c` (144 lines, the only C source).

## Mechanical grep of every rejection construct

```
$ grep -n 'assert\|RETURN_ERROR\|return -1\|return NULL\|errno\|goto' c_src/src/lib.c
(no matches)
```

The library has **no** `assert`, no error enum, no `errno`, no `RETURN_ERROR`
macro, no null checks, and no range validation. Its complete rejection surface
consists of:

1. **One explicit rejection branch**: `c2Collided`'s `default: return 0`
   (`lib.c:112-113`) — the only place the C rejects an input outright.
2. **Unguarded arithmetic** that turns malformed geometry into IEEE-754
   inf/NaN, which then always makes the final `<` comparison false and yields
   `0`. Every `return d2 < r2;` / `return d2 < r*r;` (`lib.c:72`, `lib.c:80`,
   `lib.c:101`) is such a site: GCC lowers them to `comiss` + `seta`, and
   `seta` is 0 on the unordered result, so NaN ⇒ `0`.
3. **Unchecked pointer/type reinterpretation** in `c2Collided`
   (`*(c2Circle *)A` regardless of `typeB`, `lib.c:107/109/111`).
4. **One unguarded division**: `da / c2Dot(n, n)` (`lib.c:93`) — no zero guard.
5. **Zero missing-input validation** on radii / AABB ordering / capsule
   degeneracy: all values, including negatives and inverted boxes, flow through.

Full listing of every `return` statement (the exhaustive set of exits):

```
$ grep -n 'return' c_src/src/lib.c
34,40,60  c2V / c2Mulvs / c2Sub   -> return a;            (no rejection)
44,49     c2Maxv / c2Minv         -> return c2V(...);     (ternary, NaN -> b)
54        c2Clampv                -> return c2Maxv(...);  (no clamp validation)
64        c2Dot                   -> return a.x*b.x+a.y*b.y;
72,80,101 c2Circleto*             -> return <cmp>;        (NaN -> 0)
107,109,111 c2Collided            -> dispatch
113       c2Collided              -> return 0;            <== THE rejection
143       circle_collide          -> return result;
```

## ERROR-SURFACE TABLE

One row per distinct rejection / degenerate-input condition the C actually
handles. Every row has a differential test in
`translation/tests/phase_c_errors.rs`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `c2Collided` | `typeB == 3` — first value one step past `C2_TYPE_CAPSULE` (`lib.c:112`) | `default:` arm ⇒ returns `0`; neither `A` nor `B` is dereferenced | `err01_type_one_past_max` |
| 2 | `c2Collided` | `typeB == -1` (negative, no valid variant) | `0` | `err02_type_negative_one` |
| 3 | `c2Collided` | `typeB == INT_MAX` (`2147483647`) | `0` | `err03_type_int_max` |
| 4 | `c2Collided` | `typeB == INT_MIN` (`-2147483648`) | `0` | `err04_type_int_min` |
| 5 | `c2Collided` | `typeB` = 4096 random out-of-range `int`s (any int is a legal C enum value across FFI) | `0` for every one | `err05_type_fuzz_out_of_range` |
| 6 | `c2Collided` | `A == NULL && B == NULL` with an out-of-range `typeB` (default arm is reached before any load) | `0`, no fault | `err06_null_pointers_invalid_type` |
| 7 | `c2Collided` | type confusion: `A` is reinterpreted as `c2Circle` for **every** `typeB` (`lib.c:107/109/111`), even when the caller's real `A` is an AABB/capsule — never validated | first 12 bytes of `*A` used as `c2Circle` | `err07_type_confusion_A_always_circle` |
| 8 | `c2Collided` | `typeB == 2` reads 20 bytes (`sizeof(c2Capsule)`) from `B`; a shorter buffer is not detected — verifies Rust reads exactly the same 20-byte window and no more | identical result from a 20-byte-exact buffer | `err08_capsule_reads_exactly_20_bytes` |
| 9 | `c2CircletoCapsule` | degenerate capsule `B.a == B.b` **and** `da == 0` ⇒ `0.0f / 0.0f` at `lib.c:93` (no zero guard) | quotient NaN ⇒ `d2` NaN ⇒ `NaN < r*r` false ⇒ `0` | `err09_capsule_degenerate_0_div_0` |
| 10 | `c2CircletoCapsule` | degenerate capsule `B.a == B.b` with `da > 0` ⇒ `x / +0.0f` = `+inf` at `lib.c:93` | `inf`/NaN propagates ⇒ `0` | `err10_capsule_degenerate_div_by_zero` |
| 11 | `c2CircletoCapsule` | `A.r + B.r` overflows to `+inf` (both radii `FLT_MAX`), `r*r == inf` | `d2 < inf` ⇒ `1` (no overflow check) | `err11_capsule_radius_overflow` |
| 12 | `c2CircletoCapsule` | negative radii (`A.r < 0`, `B.r < 0`) — no non-negativity check; `r*r` becomes positive | `d2 < r*r` may be `1` for a "negative-radius" circle | `err12_capsule_negative_radii` |
| 13 | `c2CircletoCapsule` | NaN in any of `A.p`, `A.r`, `B.a`, `B.b`, `B.r` ⇒ `comiss` unordered at both branch tests | `da<0` and `db<0` are both false, so the **after-B cap** arm runs. NaN in `A.p`/`A.r`/`B.b`/`B.r` reaches `d2` or `r` ⇒ `0`. NaN in **`B.a` only** feeds `n`/`ap`, which that arm never uses, so `d2 = c2Dot(A.p-B.b, A.p-B.b)` is finite and the result can legitimately be **`1`** — verified, not "fixed" | `err13_capsule_nan_inputs` |
| 14 | `c2CircletoCapsule` | ±inf coordinates ⇒ `inf - inf = NaN` inside `c2Sub` | `0` | `err14_capsule_inf_coords` |
| 15 | `c2CircletoCircle` | negative radii — `r2 = (A.r + B.r)²` is non-negative, so a negative-radius circle still "collides" | `d2 < r2` per formula, no rejection | `err15_circle_negative_radii` |
| 16 | `c2CircletoCircle` | radius sum overflows to `+inf` (`FLT_MAX + FLT_MAX`), `r2 = inf` | `1` **iff `d2` stays finite**; if the centres are far enough apart that `d2` also overflows, `inf < inf` is false ⇒ `0`. Both sub-cases asserted | `err16_circle_radius_overflow` |
| 17 | `c2CircletoCircle` | `A.r + B.r` = `+inf + -inf` ⇒ NaN | `d2 < NaN` false ⇒ `0` | `err17_circle_inf_minus_inf_radius` |
| 18 | `c2CircletoCircle` | NaN coordinate / NaN radius ⇒ unordered `comiss` | `0` | `err18_circle_nan_inputs` |
| 19 | `c2CircletoCircle` | coordinate difference overflows (`±FLT_MAX` apart) ⇒ `d2 = inf` | `inf < r2` false ⇒ `0` | `err19_circle_distance_overflow` |
| 20 | `c2CircletoAABB` | inverted AABB (`B.min > B.max` componentwise) — never validated; `c2Clampv` degenerates to `max(min, max)` | result of the formula on the inverted box | `err20_aabb_inverted_box` |
| 21 | `c2CircletoAABB` | negative `A.r` — `r2 = A.r*A.r` ≥ 0, so no rejection | per formula | `err21_aabb_negative_radius` |
| 22 | `c2CircletoAABB` | NaN in `A.p` / `A.r` / `B.min` / `B.max` ⇒ ternaries in `c2Maxv`/`c2Minv` take the `b` branch, final compare unordered | `0` | `err22_aabb_nan_inputs` |
| 23 | `c2CircletoAABB` | `A.r` = `±inf` ⇒ `r2 = +inf`; `A.r` = NaN ⇒ `r2 = NaN` | `1` for `±inf` (`(-inf)²=+inf`) **iff `d2` is finite**, else `inf < inf` is false ⇒ `0`; `0` for NaN. All three sub-cases asserted | `err23_aabb_inf_radius` |
| 24 | `c2CircletoAABB` | box with `±inf` bounds ⇒ `c2Sub` yields `inf - inf = NaN` | `0` | `err24_aabb_inf_bounds` |
| 25 | `c2Maxv` | NaN operand: `a.x > b.x` is *false* when unordered ⇒ returns **`b`**, bit-exact (incl. SNaN payload, not quieted — `movss`, not `maxss`) | `b` component bits | `err25_maxv_nan_returns_b` |
| 26 | `c2Minv` | NaN operand: `a.x < b.x` false ⇒ returns **`b`** | `b` component bits | `err26_minv_nan_returns_b` |
| 27 | `c2Maxv` / `c2Minv` | signed-zero pair `(+0.0, -0.0)` and `(-0.0, +0.0)`: `>`/`<` are both false ⇒ returns `b` (no `-0.0`→`+0.0` canonicalisation) | `b`'s sign bit | `err27_minmax_signed_zero` |
| 28 | `c2Clampv` | invalid range `lo > hi` — no ordering check; `c2Maxv(lo, c2Minv(a, hi))` ⇒ result is pulled to `lo` | per formula | `err28_clampv_lo_greater_than_hi` |
| 29 | `c2Clampv` | NaN in `a`, `lo`, or `hi` — both ternaries fall through to their `b` operand | per formula, bit-exact | `err29_clampv_nan` |
| 30 | `c2Dot` | both products NaN with *different* payloads/signs ⇒ SSE picks the destination operand; exposes GCC's exact `mulss`/`addss` operand order | destination-NaN's exact bits | `err30_dot_nan_operand_order` |
| 31 | `c2Dot` | `0 * inf` ⇒ NaN, and `inf + -inf` ⇒ NaN (default NaN, sign-negative on x86) | exact NaN bits | `err31_dot_invalid_operations` |
| 32 | `c2Dot` | SNaN input — `mulss` quiets it (sets bit 22) | quieted-NaN bits | `err32_dot_snan_quieting` |
| 33 | `c2Mulvs` | NaN scalar `b` **and** NaN `a` component with different payloads ⇒ exposes `mulss` destination choice | destination-NaN's exact bits | `err33_mulvs_nan_operand_order` |
| 34 | `c2Mulvs` | `b` = ±0 with `a` = ±inf ⇒ NaN; `b` = ±inf with `a` = ±0 ⇒ NaN | exact NaN bits | `err34_mulvs_zero_times_inf` |
| 35 | `c2Sub` | `inf - inf`, `-inf - -inf` ⇒ NaN; `-0.0 - 0.0` ⇒ `-0.0`; `0.0 - 0.0` ⇒ `+0.0` | exact bits incl. zero sign | `err35_sub_inf_and_signed_zero` |
| 36 | `c2Sub` | subtraction overflow (`FLT_MAX - -FLT_MAX`) ⇒ `+inf` | `+inf` | `err36_sub_overflow` |
| 37 | `c2V` | pass-through of every pathological bit pattern (SNaN, QNaN, `-0.0`, denormal) — no canonicalisation anywhere | identical bits out | `err37_c2v_bit_passthrough` |
| 38 | `circle_collide` | NaN `x`, `y`, or `r` — all three sub-tests return `0` | `0` | `err38_circle_collide_nan` |
| 39 | `circle_collide` | ±inf `x`/`y`/`r` | per formula (`inf` radius ⇒ bits set) | `err39_circle_collide_inf` |
| 40 | `circle_collide` | negative `r` (no non-negativity check) ⇒ `r2` positive for the circle test, `r*r` positive for the capsule test | non-zero results possible | `err40_circle_collide_negative_r` |
| 41 | `circle_collide` | `r` so large that `A.r*A.r` / `(A.r+B.r)²` overflow to `+inf` | `7` (all three bits) | `err41_circle_collide_radius_overflow` |
| 42 | all `c2*` | denormal / subnormal inputs (no flush-to-zero: both `.so`s run with default MXCSR) | identical bits | `err42_denormals` |

## Status

- [x] Rows 1–42 each have a passing differential test in
      `translation/tests/phase_c_errors.rs` (test `errNN_*` implements row NN).
- [x] All 42 pass against both the `release` and the `debug` Rust `.so`, under
      both feature combinations (`./check_features.sh`).

### Generic boundaries also covered (not distinct C branches, so not rows)

| boundary | where |
|----------|-------|
| NULL pointers | row 6 — only reachable without a fault when `typeB` is out of range; a NULL with a *valid* `typeB` faults in the C too, so it is not a testable behaviour |
| out-of-range enum across FFI | rows 1–5 — `3`, `-1`, `INT_MAX`, `INT_MIN`, and 4096 fuzzed `int`s |
| one step past a valid range | row 1 (`typeB == 3`), and the ±1-ULP probes in `cfg28`/`cfg35`/`cfg41`/`cfg44` |
| zero / oversized "lengths" | no length or count parameter exists in this API; the closest analogue is the fixed read width per `typeB`, pinned by row 8 (12 / 16 / 20 bytes exactly) |
| misaligned pointers | row 6 (never dereferenced) and `cfg51` (dereferenced at offsets 1–3) |
