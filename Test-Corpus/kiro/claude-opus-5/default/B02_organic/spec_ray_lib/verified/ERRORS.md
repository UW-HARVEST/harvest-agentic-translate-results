# ERRORS.md — error / rejection surface (Phase A, gate for Phase C)

Derived mechanically from `c_src/src/lib.c`. This library has **no** error
enum, no `errno` use, no `assert`, and no `RETURN_ERROR`-style macro. Its
entire rejection surface is:

* `int` predicates returning `0` (reject) vs `1` (accept);
* raycast functions returning `0` for "no hit" and leaving `*out` **untouched**
  (`c2RaytoCircle`, `c2RaytoAABB`) or partially written (`c2RaytoCapsule`);
* IEEE-754 sentinels produced by unguarded division / `sqrtf` (`inf`, `NaN`);
* one genuine undefined-behaviour path (`c2CastRay` with an out-of-range enum).

Every distinct rejecting branch in the source gets one row. `[x]` = a
differential test exists in `tests/errors.rs` that constructs exactly this
condition, calls **both** `.so`s, and asserts the returned int **and** the
`c2Raycast` out-parameter bytes are identical.

| # | function | trigger (exact invalid input / condition) | expected C result | [x] |
|---|----------|-------------------------------------------|-------------------|-----|
| 1 | `c2RaytoCircle` | `disc = b*b - c < 0` (ray line misses the circle) — `lib.c:100` | returns `0`; `*out` **not written** | [x] |
| 2 | `c2RaytoCircle` | `disc >= 0` but `t = -b - sqrt(disc) < 0` (both intersections behind the origin, or origin inside) — falls to `lib.c:109` | returns `0`; `*out` not written | [x] |
| 3 | `c2RaytoCircle` | `t >= 0` but `t > A.t` (hit is beyond the ray's length) — falls to `lib.c:109` | returns `0`; `*out` not written | [x] |
| 4 | `c2RaytoCircle` | `disc` is `NaN` (e.g. `A.d` or `A.p` contains `NaN`/`inf`): `NaN < 0` is false, so `sqrtf(NaN)` runs and `t` is `NaN`; `t>=0 && t<=A.t` is false | returns `0`; `*out` not written | [x] |
| 5 | `c2RaytoCircle` | `B.r < 0` (negative radius). `B.r*B.r >= 0`, so this behaves like `+B.r`; hit is still possible and `out->n` is normalised from the impact point | returns `0` or `1` identically to C | [x] |
| 6 | `c2RaytoCircle` | `A.t < 0` (negative ray length) — every `t >= 0` fails `t <= A.t` | returns `0` | [x] |
| 7 | `c2AABBtoAABB` | `d0`: `B.max.x < A.min.x` | returns `0` | [x] |
| 8 | `c2AABBtoAABB` | `d1`: `A.max.x < B.min.x` | returns `0` | [x] |
| 9 | `c2AABBtoAABB` | `d2`: `B.max.y < A.min.y` | returns `0` | [x] |
| 10 | `c2AABBtoAABB` | `d3`: `A.max.y < B.min.y` | returns `0` | [x] |
| 11 | `c2AABBtoAABB` | any coordinate `NaN` — all four `<` are false, so the "no separation" branch wins | returns `1` (accepts a NaN box) | [x] |
| 12 | `c2AABBtoAABB` | inverted box (`min > max`) — not validated anywhere | returns whatever the four `<` say | [x] |
| 13 | `c2AABBtoPoint` | `d0`: `B.x < A.min.x` | returns `0` | [x] |
| 14 | `c2AABBtoPoint` | `d1`: `B.y < A.min.y` | returns `0` | [x] |
| 15 | `c2AABBtoPoint` | `d2`: `B.x > A.max.x` | returns `0` | [x] |
| 16 | `c2AABBtoPoint` | `d3`: `B.y > A.max.y` | returns `0` | [x] |
| 17 | `c2AABBtoPoint` | `B` exactly on a face (`B.x == A.min.x`) — comparisons are strict, so on-boundary is **inside** | returns `1` | [x] |
| 18 | `c2AABBtoPoint` | `B.x` or `B.y` is `NaN` | returns `1` | [x] |
| 19 | `c2CircleToPoint` | `d2 >= A.r*A.r` (point outside) — strict `<` at `lib.c:227` | returns `0` | [x] |
| 20 | `c2CircleToPoint` | point exactly on the circle (`d2 == r*r`) | returns `0` (strict `<`) | [x] |
| 21 | `c2CircleToPoint` | `A.r == 0` — `d2 < 0` is impossible | returns `0` for every point, incl. the centre | [x] |
| 22 | `c2CircleToPoint` | `A.r < 0` — `r*r > 0`, so a negative radius behaves like a positive one | returns `1` inside | [x] |
| 23 | `c2CircleToPoint` | `NaN` in `A.p` or `B` ⇒ `d2` is `NaN` ⇒ `NaN < r*r` false | returns `0` | [x] |
| 24 | `c2RayToPlane_OneDimensional` (static, reached via `c2RaytoAABB`) | `da < 0` | returns `0.0f` | [x] |
| 25 | `c2RayToPlane_OneDimensional` | `da*db > 0` (both endpoints on the same side) | returns `1.0f` | [x] |
| 26 | `c2RayToPlane_OneDimensional` | `da == db` so `d = da-db == 0` (ray parallel to the plane) — division guarded | returns `0.0f`, **not** `inf`/`NaN` | [x] |
| 27 | `c2RaytoAABB` | swept-AABB of the ray is disjoint from `B` (`!c2AABBtoAABB`) — `lib.c:146` | returns `0`; `*out` not written | [x] |
| 28 | `c2RaytoAABB` | separating-axis reject: `\|dot(n, p0-centre)\| - dot(\|n\|, half_extents) > 0` — `lib.c:157` | returns `0`; `*out` not written | [x] |
| 29 | `c2RaytoAABB` | `hit == 0`, i.e. all four `t_i > 1.0f` — `lib.c:195` | returns `0`; `*out` not written | [x] |
| 30 | `c2RaytoAABB` | `A.t == 0` (zero-length ray): `p1 == p0`, `ab` and `n` are zero, `d = 0 - 0 = 0`, not `> 0`, so it proceeds and can report a hit with `out->t = t*0 == 0` | returns `1` when `p0` is in `B` | [x] |
| 31 | `c2RaytoAABB` | `A.d` non-normalised / zero — no validation at all | same int + same `*out` as C | [x] |
| 32 | `c2RaytoAABB` | `NaN` in `A.p`/`A.d`/`A.t`/`B`: `c2AABBtoAABB` accepts, `d > 0` is false, all `t_i <= 1` are false ⇒ `hit == 0` | returns `0` | [x] |
| 33 | `c2RaytoAABB` | inverted `B` (`B.min > B.max`) — `half_extents` negative, never checked | same as C | [x] |
| 34 | `c2RaytoCapsule` | falls through to the final `return 0` — `yAe.x*yAp.x >= 0` **and** `min(\|yAe.x\|,\|yAp.x\|) >= B.r` | returns `0`; `out->n`/`out->t` **already overwritten** with `c2Norm(b-a)` / `0` | [x] |
| 35 | `c2RaytoCapsule` | degenerate capsule `B.a == B.b`: `c2Norm(0,0)` divides by `c2Len == 0` ⇒ `1.0f/0 = inf` ⇒ `0*inf = NaN`, so `M`, `yBb`, `yAp` are all `NaN` | `out->n = (NaN, NaN)`; return value must match C bit-for-bit | [x] |
| 36 | `c2RaytoCapsule` | the slab division `t = (c - yAp.x) / (yAe.x - yAp.x)` is **unguarded**, unlike row 26. Reachability was measured, not assumed: an exact `d == 0` requires `yAe.x == yAp.x`, which makes the outer test succeed only via `min(\|yAe.x\|,\|yAp.x\|) < B.r`, i.e. `\|yAp.x\| < B.r`, which routes to the `d`/`e` circle-delegation arms first — so **0/0 is unreachable** (0 occurrences in 226 k samples that reached the arm). What *is* reachable is a denominator of `±inf` (70 k occurrences) and a non-finite quotient (7 k) | non-finite `t` propagates into `out->t`; both the unreachability of `d == 0` and the reachable `inf`/`NaN` quotients must match | [x] |
| 37 | `c2RaytoCapsule` | `B.r == 0` (zero-radius capsule) — `capsule_bb` collapses to a segment; `c2CircleToPoint` can never accept | same as C | [x] |
| 38 | `c2RaytoCapsule` | `B.r < 0` — `capsule_bb.min.x = -r > 0 > r = max.x`, an inverted box, so `c2AABBtoPoint` rejects; but `c2CircleToPoint` still uses `r*r > 0` | same as C | [x] |
| 39 | `c2RaytoCapsule` | `A.t < 0` / `A.t == 0` | same as C | [x] |
| 40 | `c2RaytoCapsule` | `NaN` in `A` or `B` | same as C | [x] |
| 41 | `c2CastRay` | `typeB == 3` — no `case`, no `default`, and **no trailing `return`**: control runs off the end of a non-`void` function (UB). At the reference build's optimisation level GCC emits `cmpl $2,typeB; ja <epilogue>` and a bare `jmp <epilogue>`; **neither edge writes `%eax`**, so the value returned is exactly the caller's incoming `%eax`. Verified by disassembly, and reproduced in Rust by a naked thunk that forwards `%eax` into the body as a synthetic 5th argument | returns the caller's incoming `%eax`, and leaves `*out` untouched. Tested with `%eax` **pinned** by a trampoline (`cast_ray_with_eax`), across 4 510 distinct injected values — an ordinary call site cannot test this, since the two call sites are not obliged to leave the same `%eax` | [x] |
| 42 | `c2CastRay` | `typeB == 0xFFFFFFFF` / `-1`, `0x80000000`, `255`, `1000`, … — the enum's underlying type is `unsigned int`, and GCC's range test is the *unsigned* `ja`, so every value `> 2` takes the same edge as row 41 | same as row 41 for every out-of-range tag | [x] |
| 43 | `c2CastRay` | `B == NULL` **and** `typeB` out of range — `B` is never dereferenced on the UB edge, and neither is `out` | no crash; both return the injected `%eax` | [x] |
| 44 | `c2CastRay` | `typeB` in range dispatches to the matching `c2Rayto*`; a *mismatched* payload size is not detectable by the C (it just reads 8/16/20 bytes) | reads whatever is there; both sides given the same over-sized buffer | [x] |
| 45 | `c2RaytoCircle` | `out == NULL` on a **miss** — `out` is only dereferenced inside the `if`, so a miss never touches it | returns `0`, no crash | [x] |
| 46 | `c2RaytoAABB` | `out == NULL` on a **miss** — same reasoning (all three early `return 0`s precede any write) | returns `0`, no crash | [x] |
| 47 | `c2Div` | `b == 0` — `1.0f/0.0f = inf`, then `a * inf` | `(±inf, ±inf)` or `NaN` for a zero component; no guard exists | [x] |
| 48 | `c2Div` | `b == NaN` / `b == inf` | `1/inf = 0` ⇒ `(0,0)`; `1/NaN = NaN` ⇒ `(NaN,NaN)` | [x] |
| 49 | `c2Norm` | zero vector — `c2Len == 0`, so `c2Div` by zero | `(NaN, NaN)` (because `0 * inf = NaN`) | [x] |
| 50 | `c2Norm` | vector with an `inf` component — `dot` overflows to `inf`, `sqrt(inf) = inf`, `1/inf = 0`, `inf*0 = NaN` | `(NaN, …)` | [x] |
| 51 | `c2Len` | component magnitude `> ~1.8e19` — `a.x*a.x` overflows to `inf` | returns `inf` | [x] |
| 52 | `c2Len` | `NaN` component | returns `NaN`; note the C calls libm `sqrtf` while Rust emits `sqrtss` — the NaN payload must still match | [x] |
| 53 | `spec_ray` | `mp == ray.p` — `c2Norm(0,0)` ⇒ `ray.d = (NaN,NaN)` ⇒ `ray.t = NaN` ⇒ `disc` `NaN` ⇒ miss | returns `0`; `*cast` not written | [x] |
| 54 | `spec_ray` | `c_r == 0` / `c_r < 0` | same as C | [x] |
| 55 | `spec_ray` | any argument `NaN` or `±inf` | same as C | [x] |
| 56 | `spec_ray` | `cast == NULL` with a guaranteed miss (circle far away) — reaches only `c2RaytoCircle`'s early `return 0` | returns `0`, no crash | [x] |
| 57 | `spec_ray` | circle behind the mouse point / ray origin inside the circle (`t < 0`) | returns `0` | [x] |
| 58 | all `c2*` float entry points | subnormal and `-0.0` inputs (`-0.0 < 0` is **false**, so `ter_abs(-0.0)` returns `-0.0`, not `+0.0`; `ter_min(-0.0, +0.0)` returns `+0.0`) | sign of zero must match bit-for-bit | [x] |
