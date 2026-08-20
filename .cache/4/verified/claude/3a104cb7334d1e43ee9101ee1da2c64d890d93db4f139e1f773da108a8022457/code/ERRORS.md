# ERRORS.md — Phase A error / rejection surface table

Mechanically derived from `c_src/src/lib.c`. Greps run over the whole C source:

```
$ grep -nE "assert|NULL|RETURN_ERROR|errno|ERROR|_MIN|_MAX" c_src/src/lib.c
   (no matches)
$ grep -n "return" c_src/src/lib.c
   34,40,44,49,54,60,64,72,80,101,107,109,111,113,143
```

The library has **no error codes, no `errno`, no asserts, no null checks and no
allocation**. Its total rejection surface is therefore:

* one *explicit* rejection: the `default:` arm of the `switch (typeB)` in
  `c2Collided` (line 112-113) returns `0`;
* three *predicate* rejections: `d2 < r2` / `d2 < r*r` (lines 72, 80, 101)
  yielding `0` when the strict `<` is false — which, per C semantics, includes
  every case where either side is NaN (`comiss` + `seta` in the compiled code);
* the implicit contract that `c2Collided`'s `A`/`B` are dereferenceable
  (`*(c2Circle *)A`), i.e. a null/short pointer is undefined behaviour in C and
  is deliberately **not** turned into a check by the Rust translation.

Everything below is a row for a *distinct* rejection branch in the C source.
"expected C result" is what the C `.so` actually returns (verified by the
differential test, never assumed).

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|----|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `c2Collided` | `typeB == 3` (one past the last valid enumerator `C2_TYPE_CAPSULE`) → `default:` | `0` | `e1_collided_type_one_past_end` | [x] |
| E2 | `c2Collided` | `typeB == -1` (one before the first enumerator `C2_TYPE_CIRCLE`) → `default:` | `0` | `e2_collided_type_negative` | [x] |
| E3 | `c2Collided` | `typeB == INT_MAX` / `INT_MIN` (extreme out-of-range `int` in the enum slot) → `default:` | `0` | `e3_collided_type_int_extremes` | [x] |
| E4 | `c2Collided` | `typeB` = every other out-of-range `int` (exhaustive sweep of `-4096..=4096` minus `{0,1,2}`, plus randomized `i32`s) → `default:` | `0` | `e4_collided_type_sweep` | [x] |
| E5 | `c2Collided` | `typeB` = out-of-range value whose **low byte** aliases a valid tag (e.g. `0x100`, `0x10000`, `0x7FFFFF01`): C compares the full `int`, so it must *not* be truncated to a valid arm | `0` | `e5_collided_type_alias_low_byte` | [x] |
| E6 | `c2CircletoCircle` | `d2 >= r2`: circles strictly separated (distance > sum of radii) → `d2 < r2` false | `0` | `e6_circle_separated` | [x] |
| E7 | `c2CircletoCircle` | `d2 == r2` exactly (tangent circles — `<` is strict, so touching is *not* a collision) | `0` | `e7_circle_exactly_tangent` | [x] |
| E8 | `c2CircletoCircle` | negative radii making `r2 = (A.r+B.r)` negative — `r2*r2` is positive again, so a "negative circle" still collides (C quirk, must be reproduced) | same as positive radii of the same magnitude | `e8_circle_negative_radii` | [x] |
| E9 | `c2CircletoCircle` | `A.r + B.r == 0` (both radii zero, or `r` and `-r`) → `r2 == 0`, `d2 < 0` impossible | `0` for every position, even coincident centres | `e9_circle_zero_radius_sum` | [x] |
| E10 | `c2CircletoCircle` | any NaN in `A.p`/`B.p`/`A.r`/`B.r` → `d2` or `r2` NaN → `comiss`/`seta` false | `0` | `e10_circle_nan_rejects` | [x] |
| E11 | `c2CircletoCircle` | infinite coordinates → `d2 = inf` (or `inf-inf = NaN`) | `0` unless `r2` is also `inf` (`inf < inf` false ⇒ still `0`) | `e11_circle_infinities` | [x] |
| E12 | `c2CircletoAABB` | point outside the box by more than `A.r` → `d2 >= r2` | `0` | `e12_aabb_outside` | [x] |
| E13 | `c2CircletoAABB` | `A.r == 0` (degenerate circle): `r2 == 0`, so `d2 < 0` impossible | `0` **even when the centre is inside the box** (`d2 == 0`) | `e13_aabb_zero_radius` | [x] |
| E14 | `c2CircletoAABB` | inverted box (`min > max` on one or both axes) — no validation in C; `c2Clampv` = `max(lo, min(a,hi))` silently returns `lo` | whatever the clamp yields (must match bit-for-bit) | `e14_aabb_inverted_box` | [x] |
| E15 | `c2CircletoAABB` | `A.r` negative → `r2 = A.r*A.r > 0`, so a negative radius still collides (C quirk) | same as `|A.r|` | `e15_aabb_negative_radius` | [x] |
| E16 | `c2CircletoAABB` | NaN in `A.p` or `A.r` → `d2`/`r2` NaN → comparison false | `0` | `e16_aabb_nan_rejects` | [x] |
| E16b | `c2CircletoAABB` | NaN in `B.min`/`B.max` — **NOT a rejection.** `c2Minv`/`c2Maxv` use `?:`, and `NaN > v` / `NaN < v` are false, so the clamp returns the *second* operand and silently **discards** the NaN bound. Verified against the compiled C library: `A={{0,-0},20.3397}`, `B.min.x=NaN` ⇒ clamp `(0, 16.6819)` ⇒ returns **`1`**. Must be reproduced, not "fixed". | `1` or `0` depending on the surviving geometry (asserted bit-exactly, and both outcomes are proven to be exercised) | `e16_aabb_nan_rejects` | [x] |
| E17 | `c2CircletoCapsule` | `d2 >= r*r` in the `da < 0` arm (query point behind `B.a`) | `0` | `e17_capsule_before_a` | [x] |
| E18 | `c2CircletoCapsule` | `d2 >= r*r` in the `db >= 0` arm (query point beyond `B.b`) | `0` | `e18_capsule_after_b` | [x] |
| E19 | `c2CircletoCapsule` | `d2 >= r*r` in the middle (`da >= 0 && db < 0`) arm | `0` | `e19_capsule_middle` | [x] |
| E20 | `c2CircletoCapsule` | degenerate capsule `B.a == B.b` → `n == (0,0)` → `c2Dot(n,n) == 0` → `da/0`. With `da == 0` this is `0/0 = NaN`; the `da<0` test is false so the middle arm divides by zero | `0`/`1` exactly as the C div-by-zero result dictates (asserted, not assumed) | `e20_capsule_degenerate_zero_length` | [x] |
| E21 | `c2CircletoCapsule` | `A.r + B.r == 0` (e.g. both zero) → `r*r == 0`, `d2 < 0` impossible | `0` even for a point exactly on the segment | `e21_capsule_zero_radius_sum` | [x] |
| E22 | `c2CircletoCapsule` | negative `A.r`/`B.r` → `r*r >= 0`, still collides (C quirk) | same as the positive-radius case | `e22_capsule_negative_radii` | [x] |
| E23 | `c2CircletoCapsule` | NaN in `A.p`, `A.r`, `B.b` or `B.r` → `da`/`db` NaN makes both `< 0` tests false, taking the *beyond-b* arm, whose `d2`/`r` are still NaN-poisoned | `0` | `e23_capsule_nan_rejects` | [x] |
| E23b | `c2CircletoCapsule` | NaN in `B.a` only — **NOT a rejection.** It poisons `n` and `ap`, so `da`/`db` are unordered and control lands in the beyond-b arm, whose `d2 = dot(A.p−B.b, A.p−B.b)` and `r = A.r+B.r` are completely NaN-free. Verified against the compiled C library: `A={{-0,0},12.15138}`, `B={{NaN,9.962195},{-2.91156,9.962195},8.850463}` ⇒ returns **`1`**. Must be reproduced. | `1` or `0` per the surviving geometry (asserted bit-exactly; the arm is asserted to be the beyond-b arm, and both outcomes are proven to be exercised) | `e23_capsule_nan_rejects` | [x] |
| E24 | `c2CircletoCapsule` | infinite capsule endpoints → `n = inf`, `da = inf` or `NaN`, and `inf*0`/`inf-inf` inside `c2Mulvs`/`c2Sub` | must match C bit-for-bit | `e24_capsule_infinities` | [x] |
| E25 | `c2Dot` | NaN operand(s) — SSE first-operand NaN-payload priority; both-NaN must keep the *same* payload the C `mulss`/`addss` chain keeps | identical NaN bits | `e25_dot_nan_payloads` | [x] |
| E26 | `c2Dot` | overflow to `±inf` and `inf + (-inf) = NaN` (products of huge magnitudes) | identical bits (`inf`, `-inf`, or NaN) | `e26_dot_overflow_to_inf` | [x] |
| E27 | `c2Mulvs` | NaN × NaN and NaN × finite — the C `mulss` keeps the *destination* (`a.x`) payload, so `a` wins over `b` | identical NaN bits | `e27_mulvs_nan_payloads` | [x] |
| E28 | `c2Sub` | `inf - inf`, `-inf - -inf` → NaN; NaN − NaN keeps the `a` payload | identical bits | `e28_sub_inf_and_nan` | [x] |
| E29 | `c2Minv`/`c2Maxv`/`c2Clampv` | NaN in either operand — the C `?:` returns the *second* operand whenever the compare is false/unordered (so **not** `f32::min`/`max` semantics) | identical bits, including `-0.0` vs `+0.0` selection | `e29_minmax_nan_and_signed_zero` | [x] |
| E30 | `circle_collide` | NaN / `±inf` / signed-zero / subnormal / huge `x`,`y`,`r`; and negative `r` (no validation in C) | identical `int` bitmask | `e30_circle_collide_extremes` | [x] |
| E31 | `c2Collided` | null `A` or `B` pointer — **UB in C** (`*(c2Circle*)A` unconditionally dereferences). Tested for parity in *child processes* so the fault is contained. Neither implementation may quietly return a value; the Rust wrapper adds no null check. **Release profile: both fault with SIGSEGV (exit 139) — exact parity.** Debug profile: rustc's `-C debug-assertions` `ub_checks` turns the null read into a non-unwinding panic (SIGABRT, exit 134) — a debug-only *diagnostic*, so only "both faulted" is required there. | fault (SIGSEGV in release, identically for C and Rust) | `e31_null_pointer_ub_parity` + `e31_null_child` | [x] |
