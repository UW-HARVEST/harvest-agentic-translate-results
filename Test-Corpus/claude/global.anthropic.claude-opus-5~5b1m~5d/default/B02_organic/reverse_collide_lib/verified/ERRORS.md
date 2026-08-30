# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. This library uses **no** error
macros, no `errno`, no `assert`, and no allocation, so its entire rejection
surface consists of:

* `default:` arms of `switch` statements (silent fall-back values),
* explicit NULL-pointer checks (`if (!ax_ptr)`, `if (cache)`, `if (outA)`, …),
* the hard iteration cap `while (iter < 20)`,
* the `FLT_EPSILON` / `FLT_MAX` guard constants,
* unguarded divisions that produce IEEE-754 `inf` / `NaN` sentinels.

Every distinct branch below is one row. `grep -n 'default:\|if (!\|if (cache)\|if (out\|if (iterations)\|iter <\|1.19209289550781250000000000000000000e-7F\|3.40282346638528859811704183484516925e+38F\|-1.0e8f' c_src/src/lib.c` reproduces the derivation.

| # | function | trigger (exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|------------------------------------------|-------------------|------|---|
| 1 | `c2Collided` | `typeA` not in {0,1,2} (e.g. 3, 4, 0x7fffffff, 0xffffffff) — C enum accepts any `int` | outer `default:` → returns `0` | `err_collided_bad_typeA` | [x] |
| 2 | `c2Collided` | `typeA == C2_TYPE_CIRCLE`, `typeB` not in {0,1,2} | inner `default:` → returns `0` | `err_collided_bad_typeB_circle` | [x] |
| 3 | `c2Collided` | `typeA == C2_TYPE_AABB`, `typeB` not in {0,1,2} | inner `default:` → returns `0` | `err_collided_bad_typeB_aabb` | [x] |
| 4 | `c2Collided` | `typeA == C2_TYPE_CAPSULE`, `typeB` not in {0,1,2} | inner `default:` → returns `0` | `err_collided_bad_typeB_capsule` | [x] |
| 5 | `c2MakeProxy` | `type` not in {0,1,2} | `switch` has **no** `default:` → `*p` left **completely untouched** (caller's bytes preserved; in `c2GJK` that means uninitialised stack) | `err_makeproxy_bad_type` | [x] |
| 6 | `c2GJKSimplexMetric` | `s->count` == 0, 1, or anything not 2/3 (incl. 4, −1, INT_MIN, INT_MAX) | `default:`/`case 1:` → returns `0.0f` | `err_simplex_metric_bad_count` | [x] |
| 7 | `c2D` | `s->count` == 3 (`case 3:`) or any other value (`default:`) | returns `c2V(0,0)` | `err_c2d_bad_count` | [x] |
| 8 | `c2Witness` | `s->count` not in {1,2,3} (0, 4, −1, INT_MAX) | `default:` → `*a = *b = c2V(0,0)`; note `den = 1/div` is still computed first, so `div == 0` is harmless here | `err_witness_bad_count` | [x] |
| 9 | `c2L` | `s->count` not in {1,2} (0, 3, 4, −1) | `default:` → returns `c2V(0,0)` | `err_c2l_bad_count` | [x] |
| 10 | `c2Witness` / `c2L` | `s->div == 0` with `count` 2 or 3 | `1.0f/0.0f = +inf` propagates → `inf`/`NaN` components, **no** trap | `err_witness_zero_div`, `err_c2l_zero_div` | [x] |
| 11 | `c2Witness` / `c2L` | `s->div` negative or denormal | signed `inf` / huge `den`, propagates | `err_witness_zero_div` | [x] |
| 12 | `c2Div` | `b == 0.0f` | `c2Mulvs(a, 1/0)` → `±inf`, or `NaN` for a zero component | `err_div_by_zero` | [x] |
| 13 | `c2Div` | `b == NaN` | all components `NaN` | `err_div_by_zero` | [x] |
| 14 | `c2Norm` | `a == (0,0)` → `c2Len == 0` → `1/0 = inf`, `0*inf` | `(NaN, NaN)` | `err_norm_zero` | [x] |
| 15 | `c2Len` | `c2Dot(a,a) < 0` impossible for finite input, but `a` containing `inf` → `inf`; `a` containing `NaN` → `NaN`; overflow of `x*x` → `inf` | `sqrtf(inf) = inf`, `sqrtf(NaN) = NaN` | `err_len_nonfinite` | [x] |
| 16 | `c2Support` | `count <= 0` (0, −1, INT_MIN) | `verts[0]` is read **unconditionally** before the loop; loop body never runs → returns `0` | `err_support_nonpositive_count` | [x] |
| 17 | `c2Support` | `d == (0,0)` or `d` containing `NaN` → every `dot > dmax` is false | returns `0` | `err_support_degenerate_d` | [x] |
| 18 | `c2GJK` | `ax_ptr == NULL` | `!ax_ptr` → `ax = c2xIdentity()` | `err_gjk_null_transforms` | [x] |
| 19 | `c2GJK` | `bx_ptr == NULL` | `!bx_ptr` → `bx = c2xIdentity()` | `err_gjk_null_transforms` | [x] |
| 20 | `c2GJK` | `outA == NULL` | `if (outA)` false → no store; return value still valid | `err_gjk_null_outputs` | [x] |
| 21 | `c2GJK` | `outB == NULL` | `if (outB)` false → no store | `err_gjk_null_outputs` | [x] |
| 22 | `c2GJK` | `iterations == NULL` | `if (iterations)` false → no store | `err_gjk_null_outputs` | [x] |
| 23 | `c2GJK` | `cache == NULL` | `if (cache)` false → cache never read, `cache_was_read = 0`, simplex reset to 1 vertex | `err_gjk_null_outputs` | [x] |
| 24 | `c2GJK` | `cache != NULL` but `cache->count == 0` | `cache_was_good = !!0 = 0` → cache **not** read, simplex reset; cache is still *written* on exit | `err_gjk_cache_count_zero` | [x] |
| 25 | `c2GJK` | `cache->count != 0` and `!(min_metric < max_metric*2 && metric < -1.0e8f)` | `cache_was_read = 1` → simplex seeded from cache (note the `metric < -1.0e8f` test makes this branch essentially always taken) | `err_gjk_cache_reject_metric` | [x] |
| 26 | `c2GJK` | `cache->count` negative | `!!count` is true → `for (i=0;i<count;++i)` never runs, but `s.count = negative`, `s.div = cache->div`; the `switch (s.count)` falls through all cases, `c2L` hits `default:`, `c2Witness` hits `default:` | `err_gjk_cache_count_negative` | [x] |
| 27 | `c2GJK` | `cache->count > 3` (e.g. 4, 8) | reads `cache->iA[i]` past the 3-element array (in-struct OOB, lands on `iB`/`div`) **and** immediately overflows the local `int saveA[3]` at `saveA[3] = …` (lib.c:428) — genuine stack corruption. **Documented UB divergence**, probed in an isolated subprocess. *Observed result: byte-IDENTICAL* on both sides (both compilers lay `saveA`/`saveB` out adjacently), but not hard-asserted because the behaviour is UB | `ub_probe` (subprocess) | [x] |
| 28 | `c2GJK` | `cache->iA[i]` / `iB[i]` outside `[0, proxy.count)` but inside `[0,8)` | indexes `pA.verts[iA]` — a *valid* in-struct read of an unwritten proxy slot. `c2MakeProxy` only writes slots `0..count`, the rest are uninitialised stack in C / zero in Rust ⇒ **documented UB divergence**, asserted only for indices `< proxy.count` | `err_gjk_cache_index_out_of_range` | [x] |
| 29 | `c2GJK` | `typeA` / `typeB` not in {0,1,2} | `c2MakeProxy` leaves the local `c2Proxy` **uninitialised** (row 5) → C reads indeterminate stack (`pA.count` may be any `int`, so `c2Support` may read arbitrarily far OOB). **Documented UB divergence**, probed in an isolated subprocess. *Observed result: C returns `dist=+0.0` with `a=b=NaN` and `iters=1`, Rust returns `dist=+0.0` with `a=b=(0,0)` and `iters=0` — because the C proxy holds caller-stack garbage while the Rust local is zeroed. Not reproducible by any translation; unreachable from every public entry point (see `ub_rows_unreachable_from_public_api`)* | `ub_probe` (subprocess) | [x] |
| 30 | `c2GJK` | shapes so far apart / degenerate that GJK never terminates early | hard cap `while (iter < 20)` → at most 20 iterations, `*iterations <= 20` | `err_gjk_iteration_cap` | [x] |
| 31 | `c2GJK` | `d1 > d0` (distance stopped decreasing; `d0` starts at `FLT_MAX`) | `break` out of the loop | covered by `err_gjk_iteration_cap` + Phase B randomisation | [x] |
| 32 | `c2GJK` | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (search direction collapsed) | `break` | `err_gjk_degenerate_shapes` | [x] |
| 33 | `c2GJK` | duplicate support point (`iA==saveA[i] && iB==saveB[i]`) | `dup = 1` → `break` before `++s.count` | covered by `err_gjk_degenerate_shapes` | [x] |
| 34 | `c2GJK` | `use_radius != 0` and `dist <= rA+rB` **or** `dist <= FLT_EPSILON` | else-branch: `a = b = midpoint`, `dist = 0` | `err_gjk_radius_shrink_to_zero` | [x] |
| 35 | `c2GJK` | `use_radius != 0`, `dist > rA+rB`, and after shrinking `a == b` exactly | `dist = 0` | `err_gjk_radius_shrink_to_zero` | [x] |
| 36 | `c2GJK` | `use_radius != 0` with **negative** proxy radius (`c2Circle.r < 0`) | `rA+rB < 0`, so `dist > rA+rB` easily holds → `dist -= rA+rB` *grows* the distance | `err_gjk_negative_radius` | [x] |
| 37 | `c2GJK` | hit case (`s.count == 3`) | `hit = 1`, `a = b`, `dist = 0` (return exactly `+0.0f`) | `err_gjk_hit_returns_zero` | [x] |
| 38 | `c2GJK` | `NaN` coordinates in either shape | every comparison is false → `c22`/`c23` take their final `else` arms, `dist` becomes `NaN`; `NaN != 0` so `c2AABBtoCapsule` reports "no collision" | `err_gjk_nan_inputs`, `err_capsule_nan` | [x] |
| 39 | `c2AABBtoCapsule` / `c2CapsuletoCapsule` | `c2GJK` returns `NaN` | `if (NaN)` is **true** in C → returns `0` | `err_capsule_nan` | [x] |
| 40 | `c2AABBtoAABB` | `min > max` (inverted/empty AABB) | plain comparisons, no validation → may still report a "hit" | `err_aabb_inverted` | [x] |
| 41 | `c2AABBtoAABB` | `NaN` in any bound | all four `<` false → `!(0) == 1` → returns **1** (reports collision) | `err_aabb_nan` | [x] |
| 42 | `c2CircletoCircle` | negative radius `A.r + B.r < 0` | `r2 = r2*r2` is **positive** → negative radii behave like positive ones | `err_circle_negative_radius` | [x] |
| 43 | `c2CircletoCircle` | `NaN` in `p` or `r` | `d2 < r2` false → returns `0` | `err_circle_nan` | [x] |
| 44 | `c2CircletoAABB` | inverted AABB (`min > max`) → `c2Clampv` = `max(lo, min(a,hi))` yields `lo` | no validation; deterministic result | `err_circle_aabb_inverted` | [x] |
| 45 | `c2CircletoAABB` | `NaN` in circle/AABB | `c2Maxv`/`c2Minv` use `>` / `<` so `NaN` propagates per the ternary; `d2 < r2` false → `0` | `err_circle_nan` | [x] |
| 46 | `c2CircletoCapsule` | degenerate capsule `B.a == B.b` → `n == (0,0)`, `da == 0` (not `< 0`), `db == 0` (not `< 0`) | takes the `bp` branch → distance to `B.b` | `err_circle_capsule_degenerate` | [x] |
| 47 | `c2CircletoCapsule` | `da >= 0 && db < 0` with `c2Dot(n,n) == 0` (only reachable with `inf`/`NaN`) | `da / 0` → `±inf`/`NaN` propagates into `d2` | `err_circle_capsule_degenerate` | [x] |
| 48 | `c2CircletoCapsule` | `NaN` in circle or capsule | `da < 0` false, `db < 0` false → `bp` branch, `d2 = NaN`, `NaN < r*r` false → `0` | `err_circle_nan` | [x] |
| 49 | `c2BBVerts` | `bb->min > bb->max` | no validation, writes 4 verts unconditionally | `err_bbverts_inverted` | [x] |
| 50 | `reverse_collide` | `r` negative | flows into `c2CircletoCircle` (row 42) and `c2CircletoAABB` (`r*r > 0`) and `c2CircletoCapsule` | `err_reverse_collide_negative_r` | [x] |
| 51 | `reverse_collide` | `x`/`y`/`r` = `NaN`, `±inf`, `FLT_MAX`, denormal, `±0` | must return the identical `int` bitmask | `err_reverse_collide_nonfinite` | [x] |
| 52 | `c2Maxv` / `c2Minv` / `c2Clampv` | `NaN` operand — ternary `a.x > b.x ? a.x : b.x` returns `b.x` when either is `NaN` | asymmetric, `NaN`-order-dependent result | `err_minmax_nan` | [x] |
| 53 | `c2Dot` | two operands that are both NaN with **different** sign/payload (e.g. `a=(+QNaN₁,−QNaN₂)`) | gcc emits `mulss %xmm0,%xmm1` / `mulss %xmm2,%xmm0` / `addss %xmm1,%xmm0`, i.e. the **`a.y*b.y` product is the `addss` destination and therefore wins**. Fixed in Rust with an explicit `addss` (`ss_add(ss_mul(b.y,a.y), ss_mul(a.x,b.x))`) | `nan_order_vec_binary` | [x] |
| 54 | `c2Det2` | ditto | `subss` dst = `b.y*a.x`, src = `b.x*a.y` | `nan_order_vec_binary` | [x] |
| 55 | `c2Add` | ditto | `addss %xmm1,%xmm0` with dst = **`b.x`**, src = `a.x` (reversed w.r.t. the C source order) | `nan_order_vec_binary` | [x] |
| 56 | `c2Sub` / `c2Mulvs` / `c2Div` | ditto | dst = `a.x` / dst = `a.x` / `divss` dst = `1.0f` | `nan_order_vec_binary`, `nan_order_vec_scalar` | [x] |
| 57 | `c2Mulrv` | rotor and vector both containing distinct NaNs | x: `subss(mulss(b.x,a.c), mulss(b.y,a.s))`; y: `addss(mulss(a.s,b.x), mulss(b.y,a.c))` | `nan_order_rotations` | [x] |
| 58 | `c2MulrvT` | ditto | x: `addss(mulss(a.c,b.x), mulss(b.y,a.s))`; y: `addss(mulss(xorps(a.s),b.x), mulss(b.y,a.c))` — note `xorps` flips the NaN sign **without** quieting | `nan_order_rotations` | [x] |
| 59 | `c22` / `c23` | simplex points containing distinct NaNs | `div` sums are left-associative with the natural dst order (`ss_add(u,v)`, `ss_add(ss_add(uABC,vABC),wABC)`); `uABC = ss_mul(det2, area)` | `nan_order_simplex` | [x] |
| 60 | `c2Witness` / `c2L` | `u` fields / `div` containing distinct NaNs | gcc emits `mulss den(%rbp),%xmm0` with `%xmm0 = u`, so **dst = `u`, src = `den`** — reversed w.r.t. the C source `(den * s->a.u)` | `nan_order_simplex` | [x] |
| 61 | `c2CircletoCircle` / `c2CircletoCapsule` | `A.r`/`B.r` both NaN with different payloads | `r = addss` with dst = **`B.r`**, src = `A.r` (reversed) | `nan_order_predicates` | [x] |
| 62 | `c2GJK` | NaN radii / coordinates reaching the `use_radius` block | `rA + rB` has dst = `rA`; `dist -= rA+rB` has dst = `dist`; `max_metric * 2.0f` is emitted as `addss %xmm0,%xmm0` | `nan_order_gjk` | [x] |
| 63 | `c2Len` | NaN or SNaN argument | `sqrtf`/`sqrtss` return the argument with the quiet bit forced on, sign and payload preserved | `nan_order_unary_vec` | [x] |
| 64 | `c2Neg` / `c2Skew` / `c2CCW90` | SNaN argument | gcc emits `xorps` against `0x80000000`, which flips the sign but does **not** quiet an SNaN | `nan_order_unary_vec` | [x] |

## NaN operand-order note (rows 53-64)

These rows are the reason `translation/src/lib.rs` routes every float operation
through the `ss_add` / `ss_sub` / `ss_mul` / `ss_div` / `ss_sqrt` helpers, which
emit the instruction with `core::arch::asm!`. Writing the rule as ordinary Rust
(`if dst.is_nan() { quiet(dst) } else { dst + src }`) does **not** work: LLVM IR
leaves NaN payload propagation unspecified, so instcombine folds the guard away
and restores its own operand order — which was observed to differ between the
`dev` and `release` profiles. Inline assembly is the only way to pin both the
instruction and its operand order. Verified by disassembling both `.so` files.

## Documented UB divergences (rows 5, 28, 29)

`c2MakeProxy` has no `default:` arm, so `c2GJK` with an out-of-range `C2_TYPE`
reads an **uninitialised** `c2Proxy` from the C stack. That value is
indeterminate and cannot be reproduced by any translation (the Rust local is
zero-initialised). These rows are therefore tested for *"same class of
behaviour, no crash, no trap"* rather than byte equality, and they are
unreachable from every public entry point (`c2Collided`, the `c2*to*`
predicates and `reverse_collide` all filter the enum through a `default:` arm
first). Row 28 is asserted byte-for-byte for all cache indices that
`c2MakeProxy` actually wrote (`0 .. proxy.count`).
