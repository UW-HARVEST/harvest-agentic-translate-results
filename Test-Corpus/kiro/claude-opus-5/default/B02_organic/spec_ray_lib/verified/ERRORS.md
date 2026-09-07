# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping every `return`, every
`if`, every comparison against a constant, and every unguarded arithmetic
operation. Command used to enumerate candidate sites:

```sh
grep -n 'return\|assert\|NULL\|if (' c_src/src/lib.c
```

## What the C does *not* have

Establishing this is part of the table, because the absence of a check is
itself behaviour the Rust must reproduce:

| grep | matches |
|------|---------|
| `assert` | 0 — no assertions anywhere |
| `NULL` / `nullptr` | 0 — **no null-pointer check on any `c2Raycast *out`** |
| `errno`, error enum, `RETURN_ERROR` macro | 0 — no error enum exists |
| `return -1` | 0 — the only integer returns are `0` and `1` |
| `if (b == 0)` before a divide | 0 — `c2Div`, `c2Norm` divide unguarded |

So there is **no error code channel**. The rejection surface is (a) the
`int` 0/1 predicate-and-hit returns, (b) unguarded arithmetic that yields
`inf`/`NaN` sentinels instead of rejecting, (c) one control-flow path that
falls off the end of a non-void function, and (d) unchecked pointer
dereference. All four kinds are enumerated below.

Legend for "expected C result": `ret` is the `int` return value; `*out` says
whether the C writes through the out-pointer on that path.

## The table

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|-----------------------------------------|-------------------|------|
| 1 | `c2RaytoCircle` | `disc < 0` (line 100) — ray line misses the circle: `b*b - (dot(m,m) - r*r) < 0` | `ret 0`, `*out` **untouched** | `err_01_raytocircle_disc_negative` |
| 2 | `c2RaytoCircle` | `t < 0` (line 103, first conjunct fails) — nearest root behind the ray origin, i.e. origin strictly inside or past the circle | `ret 0`, `*out` untouched | `err_02_raytocircle_t_negative` |
| 3 | `c2RaytoCircle` | `t > A.t` (line 103, second conjunct fails) — hit exists but beyond the ray's length | `ret 0`, `*out` untouched | `err_03_raytocircle_t_beyond_len` |
| 4 | `c2RaytoCircle` | `r == 0` — degenerate zero-radius circle; `c = dot(m,m)`, essentially never hit | `ret 0` unless the ray line passes exactly through `B.p` | `err_04_raytocircle_zero_radius` |
| 5 | `c2RaytoCircle` | `r < 0` — negative radius; C squares it (`B.r*B.r`), so it behaves as `|r|`. No rejection. | same `ret` as `+r` | `err_05_raytocircle_negative_radius` |
| 6 | `c2RaytoCircle` | `A.d` is not unit length (C never normalises or validates it) | no rejection; `t` scaled by `1/|d|` | `err_06_raytocircle_nonunit_dir` |
| 7 | `c2RaytoCircle` | `A.t < 0` — negative ray length makes `t <= A.t` unsatisfiable for `t >= 0`, except `t == A.t == 0` is impossible | `ret 0`, `*out` untouched | `err_07_raytocircle_negative_t` |
| 8 | `c2RaytoCircle` | any input component `NaN` — `disc < 0` is false for `NaN`, then `t >= 0` is false | `ret 0`, `*out` untouched | `err_08_raytocircle_nan_inputs` |
| 9 | `c2RaytoCircle` | hit exactly at `t == 0` (origin on the circle surface, ray outbound) | `ret 1`; `out->n` = `c2Norm(0,0)` = `NaN`/`inf` pattern | `err_09_raytocircle_t_exactly_zero` |
| 10 | `c2RaytoCircle` | hit exactly at `t == A.t` (boundary of the inclusive `<=`) | `ret 1` (accepted, not rejected) | `err_10_raytocircle_t_exactly_len` |
| 11 | `c2RaytoAABB` | swept-AABB vs `B` disjoint → `!c2AABBtoAABB` (line 145) | `ret 0`, `*out` untouched | `err_11_raytoaabb_broadphase_reject` |
| 12 | `c2RaytoAABB` | separating-axis test `d > 0` (line 156) — overlapping bounds but the ray's line misses the box | `ret 0`, `*out` untouched | `err_12_raytoaabb_sat_reject` |
| 13 | `c2RaytoAABB` | `hit == 0` (line 195) — all four `t0..t3` fail `<= 1.0`. **Only reachable with a `NaN`**: for finite inputs the helper returns `0`, `1.0f`, or `da/(da-db)` with `da > 0 >= db`, so the quotient is in `[0,1]` and `t <= 1.0` always holds. A `NaN` origin makes all four `t` values `NaN`, and `NaN <= 1.0` is false. | `ret 0`, `*out` untouched | `err_13_raytoaabb_no_slab_hit` |
| 14 | `c2RaytoAABB` | inverted box `B.min > B.max` on one or both axes — no validation; `half_extents` goes negative | no rejection; whatever the SAT math yields | `err_14_raytoaabb_inverted_box` |
| 15 | `c2RaytoAABB` | degenerate box `B.min == B.max` (zero extent) | no rejection; follows the same path | `err_15_raytoaabb_degenerate_box` |
| 16 | `c2RaytoAABB` | `A.t == 0` — `p1 == p0`, `ab == 0`, `n == 0`, so `d = -dot(0,half) = -0` → `d > 0` false, continues | `ret 1` with `out->t == 0*A.t` | `err_16_raytoaabb_zero_length_ray` |
| 17 | `c2RaytoAABB` | `NaN` in `A.p`/`A.d`/`B` — `c2AABBtoAABB`'s `<` are all false, so the broad phase *accepts*; `d > 0` is false; `t <= 1.0` is false for `NaN` | reaches the slab stage and rejects there (`hit == 0`) unless a non-`NaN` axis hits | `err_17_raytoaabb_nan_inputs` |
| 18 | `c2RayToPlane_OneDimensional` (via `c2RaytoAABB`) | `da < 0` (line 126) — segment start behind the plane | returns `0` for that slab | `err_18_ray2plane_da_negative` |
| 19 | `c2RayToPlane_OneDimensional` (via `c2RaytoAABB`) | `da * db > 0` (line 128) — both endpoints on the same side. **Dead code as reached from `c2RaytoAABB`**: `da0 > 0 && db0 > 0` means both endpoints are left of `B.min.x`, hence `a_box.max.x < B.min.x`, hence the broad phase at line 145 already returned 0 (symmetrically for the other three planes). Verified by search in `err_19_ray2plane_same_side`, which reports its own counts: the condition arises 50 162 times in 60 000 inputs and **0** times after the broad phase passes. | would return `1.0f`; never observable | `err_19_ray2plane_same_side` (asserts the unreachability) |
| 20 | `c2RayToPlane_OneDimensional` (via `c2RaytoAABB`) | `d == da - db == 0` (line 132 false) — divide-by-zero guard, the **only** explicit zero-divisor check in the library | returns `0` instead of `da/0` | `err_20_ray2plane_zero_denominator` |
| 21 | `c2AABBtoAABB` | `NaN` in either box — every `<` is false so `d0..d3 == 0` | `ret 1` ("overlapping"), not a rejection | `err_21_aabbtoaabb_nan` |
| 22 | `c2AABBtoPoint` | point strictly outside on any of 4 sides (line 222) | `ret 0` | `err_22_aabbtopoint_outside` |
| 23 | `c2AABBtoPoint` | `NaN` coordinate — all 4 comparisons false | `ret 1`, not a rejection | `err_23_aabbtopoint_nan` |
| 24 | `c2CircleToPoint` | `d2 >= r*r` (line 228) — point on or outside the circle; note the boundary is **exclusive**, so a point exactly on the rim is rejected | `ret 0` | `err_24_circletopoint_on_or_outside` |
| 25 | `c2CircleToPoint` | `r == 0` — `d2 < 0` is never true | `ret 0` always | `err_25_circletopoint_zero_radius` |
| 26 | `c2CircleToPoint` | `NaN` coordinate — `d2 < r*r` is false | `ret 0` | `err_26_circletopoint_nan` |
| 27 | `c2RaytoCapsule` | fall-through `return 0` (line 291) — ray stays outside the slab and never crosses `±B.r` | `ret 0`, but `*out` **was already written** at lines 243–244 (`out->n = c2Norm(cap_n)`, `out->t = 0`) | `err_27_raytocapsule_fallthrough_writes_out` |
| 28 | `c2RaytoCapsule` | degenerate capsule `B.a == B.b` — `c2Norm(0,0)` divides by zero → `M.y` is `NaN`, poisoning `M`, `yBb`, `yAp` | no rejection; `out->n` is `NaN`; `ret` follows the `NaN`-comparison paths | `err_28_raytocapsule_degenerate_ab` |
| 29 | `c2RaytoCapsule` | `B.r == 0` — `capsule_bb` is a zero-width slab; `abs(yAp.x) < 0` and `min(...) < 0` are both false, so every early accept and both cap delegates become unreachable and the capsule degenerates to its **axis segment**. A ray aimed at the axis therefore still `ret 1`; only rays that cross `x == 0` outside `[0, yBb.y]` (or never cross) `ret 0`. | both outcomes occur; not a blanket reject | `err_29_raytocapsule_zero_radius` |
| 30 | `c2RaytoCapsule` | `B.r < 0` — negative radius; `capsule_bb.min.x = -r > 0 = capsule_bb.max.x` is inverted, and `c2CircleToPoint` squares `r` | no rejection; inconsistent-but-defined behaviour | `err_30_raytocapsule_negative_radius` |
| 31 | `c2RaytoCapsule` | origin inside the slab (line 245) or inside either end cap (lines 254/256) | early `ret 1` with `out->t == 0` and `out->n == c2Norm(b-a)` — **not** the true surface normal | `err_31_raytocapsule_origin_inside` |
| 32 | `c2RaytoCapsule` | `d = yAe.x - yAp.x` at line 278 is an **unguarded** divide, unlike line 132. An *exactly zero* `d` turns out to be unreachable here: line 278 requires `abs(yAp.x) >= B.r` plus either `yAe.x*yAp.x < 0` (opposite signs ⇒ different) or `min(|yAe.x|,|yAp.x|) < B.r <= |yAp.x|` (⇒ `|yAe.x| != |yAp.x|`). A **non-finite** `d` (`inf - inf`, or a `NaN` operand) *is* reachable and is the same missing-guard defect. | `t` is `NaN`/`±inf`; `y` then selects a cap or the `ret 1` branch | `err_32_raytocapsule_zero_denominator` |
| 33 | `c2RaytoCapsule` | delegated rejection — the chosen `c2RaytoCircle(A, Ca/Cb, out)` itself returns 0 (lines 272/274/281/283) | `ret 0`, `*out` holds the line 243–244 values (the delegate left it untouched) | `err_33_raytocapsule_delegate_rejects` |
| 34 | `c2CastRay` | `typeB` outside `{0,1,2}` — the `switch` has **no `default`** and no trailing `return`; control falls off the end of a non-void function (line 303) | UB: the compiled C executes a bare `leave; ret` that never writes `%eax`, so the caller observes **its own leftover `%eax`**; `*out` untouched | `err_34_castray_out_of_range_tag` |
| 35 | `c2CastRay` | the unreachable `return 0;` at line 302, dead after the `c2RaytoCapsule` return | never executed; documents that even the author's intended default is `0` | covered by #34 |
| 36 | `c2CastRay` | `typeB` valid but `B` points at a *smaller* object than the tag implies (e.g. tag `CAPSULE`, a `c2Circle` behind `B`) — no size/tag validation | reads past the object; no rejection | `err_36_castray_tag_shape_mismatch` |
| 37 | `c2CastRay` / `c2Rayto*` / `spec_ray` | `out == NULL` | **no null check anywhere**; the C dereferences and faults — except on paths that return before any write (#1, #2, #3, #11, #12, #13, #34), where `NULL` is harmless | `err_37_null_out_on_early_return_paths` |
| 38 | `spec_ray` | `mp == (r_p_x, r_p_y)` — `c2Norm(0,0)` divides by zero, `ray.d` is `NaN`, `ray.t` is `NaN` | `ret 0` (all comparisons in `c2RaytoCircle` false), `*out` untouched | `err_38_specray_mp_equals_origin` |
| 39 | `spec_ray` | mouse point behind the circle so `ray.t` is too short | `ret 0` | `err_39_specray_ray_too_short` |
| 40 | `spec_ray` | `c_r == 0` / `c_r < 0` / `NaN` in any of the 7 floats | `ret 0` on the degenerate cases; `NaN` never hits | `err_40_specray_degenerate_scalars` |
| 41 | `c2Div` / `c2Norm` | `b == 0` (`c2Div`) or `|a| == 0` (`c2Norm`) — unguarded `1.0f / b` | `±inf` components, or `NaN` for `0 * inf`; sign of zero decides `+inf` vs `-inf` | `err_41_div_norm_by_zero` |
| 42 | `c2Div` / `c2Norm` | `b == NaN`, `b == ±inf` | `1/inf == 0` → zero vector; `1/NaN == NaN` → `NaN` vector | `err_42_div_norm_nonfinite` |
| 43 | `c2Len` | `c2Dot(a,a) == inf` (overflow on squaring, e.g. `a.x == f32::MAX`) | `sqrtf(inf) == inf`, no rejection | `err_43_len_overflow` |
| 44 | `c2Len` | `a` contains `NaN` → `sqrtf(NaN)` | `NaN` (no domain error path; the argument can never be negative) | `err_44_len_nan` |
| 45 | `c2Minv`/`c2Maxv`/`c2Absv` | `NaN` operand — the C uses ternaries, not `fminf`/`fmaxf`/`fabsf`, so `NaN < x` is false and the **second** operand is selected by `c2Minv`; `c2Absv(NaN)` returns `NaN` with its **sign bit intact** | ternary semantics, not libm semantics | `err_45_ternary_nan_semantics` |
| 46 | `c2Minv`/`c2Maxv`/`c2Absv` | `-0.0` operand — `-0.0 < 0.0` is false, so `c2Absv(-0.0)` returns `-0.0` (`fabsf` would give `+0.0`), and `c2Minv(-0.0, +0.0)` returns the second operand | sign of zero preserved/selected per ternary | `err_46_ternary_signed_zero` |

Row 35 is documentation-only (provably dead code, no input can reach it). Row
19 is a proven-unreachable branch whose test asserts that unreachability rather
than pretending to exercise it. Every other row has a dedicated differential
test in `translation/tests/errors.rs`, and each test also asserts that the C
really takes the intended branch, so a row cannot pass vacuously.

## Result

All 45 tests in `translation/tests/errors.rs` pass against both the release-
and debug-built Rust `.so`, under both `default` and `--no-default-features`.

One divergence was found and fixed during Phase C, in row 34. The original
translation returned `B as usize as c_int` for an out-of-range tag, on the
theory that the compiled C returns its first integer argument. The disassembly
shows otherwise: the C's fall-through is a bare `leave; ret` that never writes
`%eax`, so the caller observes *its own* leftover `%eax`. `c2CastRay` is now an
`#[unsafe(naked)]` stub that performs only the `cmp esi, 2` / `ja` range check
the C performs and then either tail-jumps to the real body or `ret`s with
`%eax` untouched. An ordinary Rust body cannot do this: with optimisations off
the prologue spills the memory-passed `c2Ray` argument through `%rax` before any
user code runs, which is why an earlier `asm!`-based attempt matched in release
but not in debug.

## Generic FFI-boundary boundaries also covered

Required by Phase C independently of the table above; all in
`translation/tests/errors.rs`:

* **Out-of-range enum across FFI** — `c2CastRay` with `typeB` = `3`, `4`, `7`,
  `100`, `0x7FFF_FFFF`, `0xFFFF_FFFF` (row #34).
* **Null pointers** — `out == NULL` on every path that returns before writing
  (row #37). Paths that *do* write are not exercised with `NULL`: the C
  segfaults there, which is not a comparable observable.
* **Zero lengths** — `A.t == 0` for all three raycasts, `B.r == 0`,
  `B.min == B.max`, `B.a == B.b`.
* **Oversized values** — `f32::MAX`, `-f32::MAX`, `±inf` for ray length,
  radius, and every coordinate.
* **One step past a valid range** — `next_after` around the accept/reject
  boundaries `t == 0`, `t == A.t`, `d2 == r*r`, `t == 1.0`,
  `disc == 0` (tangent).
