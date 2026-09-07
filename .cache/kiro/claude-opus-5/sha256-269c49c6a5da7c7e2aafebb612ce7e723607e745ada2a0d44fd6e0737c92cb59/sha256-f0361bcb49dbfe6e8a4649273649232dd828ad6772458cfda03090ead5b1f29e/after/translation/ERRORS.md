# ERRORS.md — Phase C error / rejection surface table

Mechanically derived from `c_src/src/lib.c`. This library has no error enum, no
`assert`, no `RETURN_ERROR` macro and no `NULL` returns; its ONLY rejection
channel is the `int` return value, where **0 == "no hit / reject"** and
**1 == "hit / accept"**. Every distinct `return 0` / falsy path in the C, plus
every degenerate-value producing check, gets one row.

Line numbers refer to `c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `c2RaytoCircle` | L100 `disc < 0`: ray line misses the circle entirely (`b*b - (dot(m,m)-r*r) < 0`) | returns `0`, `*out` left UNTOUCHED |
| 2 | `c2RaytoCircle` | L103/109 `t < 0`: nearest root behind the ray origin (origin past/inside circle) | returns `0`, `*out` untouched |
| 3 | `c2RaytoCircle` | L103/109 `t > A.t`: intersection farther than the ray length | returns `0`, `*out` untouched |
| 4 | `c2RaytoCircle` | `r == 0` → `c = dot(m,m)`, disc `= b*b - dot(m,m)`; degenerate point circle | `0` unless the ray line passes exactly through `B.p` |
| 5 | `c2RaytoCircle` | `r < 0` → `r*r > 0`, so a negative radius behaves like `|r|` (C does not reject it) | same as `r = -r`; NOT rejected |
| 6 | `c2RaytoCircle` | `A.t < 0` → `t <= A.t` can never hold for `t >= 0` except `t == A.t == 0` | returns `0` |
| 7 | `c2RaytoCircle` | any NaN in `A.p`/`A.d`/`B.p`/`B.r` → `disc` NaN → `disc < 0` false, then `t` NaN → `t >= 0` false | returns `0` (falls to L109), `*out` untouched |
| 8 | `c2AABBtoAABB` | L112 `B.max.x < A.min.x` (separated on -x) | returns `0` |
| 9 | `c2AABBtoAABB` | L113 `A.max.x < B.min.x` (separated on +x) | returns `0` |
| 10 | `c2AABBtoAABB` | L114 `B.max.y < A.min.y` (separated on -y) | returns `0` |
| 11 | `c2AABBtoAABB` | L115 `A.max.y < B.min.y` (separated on +y) | returns `0` |
| 12 | `c2AABBtoAABB` | NaN in any coordinate → every `<` is false → `!(0)` | returns `1` (accepts!) |
| 13 | `c2RaytoAABB` | L145 `!c2AABBtoAABB(a_box, B)`: ray's own bounding box does not overlap `B` | returns `0`, `*out` untouched |
| 14 | `c2RaytoAABB` | L156 `d > 0`: separating-axis test on the ray's skew normal fails | returns `0`, `*out` untouched |
| 15 | `c2RaytoAABB` | L175/195 `hit == 0`, i.e. all four `tN > 1.0` | returns `0`, `*out` untouched |
| 16 | `c2RaytoAABB` | `B.min > B.max` (inverted / empty box) — C never validates this | no rejection; whatever the four plane tests yield |
| 17 | `c2RaytoAABB` | `A.t == 0` → `p1 == p0`, `ab == 0`, `n == 0`, `d = -dot(0,half) <= 0` | not rejected by `d>0`; `out->t = tN * 0` |
| 18 | `c2RaytoAABB` | NaN in a coordinate. `c2AABBtoAABB` returns 1 (row 12) and `d > 0` is false, so control ALWAYS reaches the four plane tests. `c2RayToPlane_OneDimensional(da, db)` only yields NaN when **`da` itself** is NaN: with `da` NaN, `da<0` is false, `da*db>0` is false, `d = da-db` is NaN, `NaN != 0` is TRUE, so it returns `NaN/NaN = NaN` and `tN <= 1` is false. `da` comes from `p0` only. Therefore: **NaN in `A.d` or `A.t` poisons only `db` (via `p1`), leaves every `da` finite, and the C ACCEPTS (returns 1) and WRITES `*out`.** NaN in one axis of `A.p` poisons only that axis's two planes, so the other axis still gives finite `tN` → also ACCEPTS. Only NaN in **both** axes of `A.p` poisons all four `da` → `hit == 0` → reject | `1` (writing `*out`) for NaN in `A.d`, `A.t`, or a single axis of `A.p`; `0` only when both axes of `A.p` are NaN |
| 19 | `c2RayToPlane_OneDimensional` (private, reached via `c2RaytoAABB`) | L126 `da < 0` (start point already outside that plane's negative side) | contributes `t = 0` |
| 20 | `c2RayToPlane_OneDimensional` | L132/135 `da - db == 0` with `da*db <= 0` (ray parallel to the plane, `da==db`, incl. `da==db==0`) | contributes `t = 0` |
| 21 | `c2AABBtoPoint` | L200 `B.x < A.min.x` | returns `0` |
| 22 | `c2AABBtoPoint` | L201 `B.y < A.min.y` | returns `0` |
| 23 | `c2AABBtoPoint` | L202 `B.x > A.max.x` | returns `0` |
| 24 | `c2AABBtoPoint` | L203 `B.y > A.max.y` | returns `0` |
| 25 | `c2AABBtoPoint` | NaN coordinate → all four comparisons false | returns `1` (accepts!) |
| 26 | `c2CircleToPoint` | L209 `d2 >= A.r * A.r` — note `>=`, a point exactly ON the rim is rejected | returns `0` |
| 27 | `c2CircleToPoint` | `A.r == 0` → `d2 < 0` impossible | always returns `0` |
| 28 | `c2CircleToPoint` | NaN in `A.p`/`B`/`A.r` → `d2 < r*r` false | returns `0` |
| 29 | `c2RaytoCapsule` | L291 final `return 0`: `yAe.x*yAp.x >= 0` AND `min(|yAe.x|,|yAp.x|) >= B.r` (ray stays on one side, never within the slab) | returns `0`, but `*out` HAS ALREADY BEEN WRITTEN at L243-244 (`n = norm(b-a)`, `t = 0`) |
| 30 | `c2RaytoCapsule` | degenerate capsule `B.a == B.b` → `c2Norm(0,0)` = `0 * (1/0)` = NaN → `M` all NaN → `yBb`,`yAp`,`yAe` NaN; `out->n` = NaN,NaN, `out->t = 0`; `c2AABBtoPoint` on NaN returns 1 (row 25) | returns `1` with `out->n = (NaN, NaN)`, `out->t = 0` |
| 31 | `c2RaytoCapsule` | `B.r == 0` → `capsule_bb` is the degenerate segment box; `|yAp.x| < 0` false; `min(...) < 0` false → needs `yAe.x*yAp.x < 0` | usually `0`; when taken, `c = -0.0` so `c > 0` false → `n = c2Skew(M.y)` |
| 32 | `c2RaytoCapsule` | `B.r < 0` → `capsule_bb.min.x = -B.r > 0 = capsule_bb.max.x`, an inverted box; `c2CircleToPoint` with `r<0` uses `r*r` | no rejection; C runs the inverted-box path |
| 33 | `c2RaytoCapsule` | NaN in `A.p`/`A.d`/`A.t` → `yAp` NaN → `c2AABBtoPoint` returns 1 | returns `1`, `out->t = 0`, `out->n = norm(b-a)` |
| 34 | `c2RaytoCapsule` | delegated rejection: `|yAp.x| < B.r` and the chosen end circle misses → `c2RaytoCircle` returns 0 | returns `0`, `*out` keeps the L243-244 values |
| 35 | `c2RaytoCapsule` | delegated rejection: `y <= 0` / `y >= yBb.y` end-cap path where `c2RaytoCircle` misses | returns `0`, `*out` keeps L243-244 values |
| 36 | `c2CastRay` | L295 `typeB` is not 0/1/2 (e.g. `3`, `-1`, `255`, `INT_MIN`, `INT_MAX`). The `switch` has NO `default`, so C falls off the end of a non-`void` function. GCC compiles the fall-through to `leave; ret` with **`%eax` never written**, so the value the caller observes is its own incoming `%rax`; `*out` is never touched. Verified by disassembly (`cmpl $0x2,typeB; ja .Lout; … .Lout: leave; ret`) | the caller's incoming return register, verbatim, and `*out` unchanged. The Rust reproduces this with a `#[naked]` range-check shim that tail-jumps for 0/1/2 and bare-`ret`s otherwise. **Tested through an asm trampoline that seeds `%rax`**, because two adjacent Rust call statements do NOT guarantee the same entry state — a naive side-by-side call compares the harness's register allocation, not the libraries |
| 37 | `c2CastRay` | `typeB` invalid AND `B == NULL` AND `out == NULL`: no dereference happens on this path | must not crash; same result as row 36 |
| 38 | `c2CastRay` | `out == NULL` with `typeB = C2_TYPE_CIRCLE` on a rejecting ray (row 1) — `out` is never written | returns `0`, no crash |
| 39 | `c2CastRay` | `out == NULL` with `typeB = C2_TYPE_AABB` on a rejecting ray (row 13) — `out` is never written | returns `0`, no crash |
| 40 | `c2Div` / `c2Norm` | `b == 0` → `1.0f/0.0f = +inf`; `c2Norm` of the zero vector = `0 * inf` | `(NaN, NaN)` — not rejected, propagated |
| 41 | `c2Norm` | input containing NaN or inf | NaN/inf propagated, no rejection |
| 42 | `gen_ray` | `mp == ray.p` → `c2Norm(0,0)` = `(NaN, NaN)` direction and `ray.t = NaN`. `ray.p` itself stays finite, so each sub-cast reacts differently: the circle always rejects (row 7); the capsule's `yAp` is still finite (only `yAe` goes NaN) so it may accept or reject; the AABB's `da` values are still finite (row 18) so it usually accepts | mask is data-dependent — measured over 30 000 randomized cases: `{0, 2, 4, 6}` all occur (`4` dominates). Must match bit-for-bit, including all three `c2Raycast` outs |
| 43 | `gen_ray` | all-zero float arguments (degenerate everything) | deterministic value; must match bit-for-bit incl. the three `c2Raycast` outs |
| 44 | `gen_ray` | any NaN/inf float argument | no validation in C; must match bit-for-bit |

## Notes on what is NOT a testable row

* `out == NULL` on a *hit* path, and `B == NULL` with a *valid* `typeB`, are
  unconditional null dereferences in the C (segfault). They are genuine UB with
  no defined C result, so they are excluded — rows 37-39 cover exactly the null
  cases the C actually survives.
* There is no allocation, no I/O and no length/size parameter anywhere in this
  library, so there are no "oversized length" or allocation-failure rows.
