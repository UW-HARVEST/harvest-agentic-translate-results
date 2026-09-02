# ERRORS.md — Phase A error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping every `return 0`,
`return 1`, `return !(...)`, every `if` that guards a return, every `assert`
(there are **none**), every null check, and every implicit range/domain limit.
Line numbers refer to `c_src/src/lib.c`.

Note on the C's error convention: this library has **no error enum, no
`errno`, no `RETURN_ERROR` macro and no `assert`**. Every rejection is
signalled by returning the `int` sentinel `0` ("no hit" / "no overlap"), and
`1` means hit. The only *pointer* check in the whole library is the
`bx_ptr ? *bx_ptr : c2xIdentity()` null test in `c2RaytoPoly`. Rows below
therefore enumerate every distinct sentinel-`0` branch plus the domain-edge
conditions that produce non-finite output instead of a sentinel.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| 1  | `c2RaytoCircle` (L117-118) | `disc = b*b - c < 0` — ray line misses the circle entirely | returns `0`, `*out` untouched |
| 2  | `c2RaytoCircle` (L121,126) | `t = -b - sqrtf(disc)` is `< 0` (circle behind ray origin / origin inside) | returns `0`, `*out` untouched |
| 3  | `c2RaytoCircle` (L121,126) | `t > A.t` — hit exists but beyond ray length | returns `0`, `*out` untouched |
| 4  | `c2RaytoCircle` | `B.r < 0` (negative radius): `B.r*B.r` is used, so behaves like `+r`; but `t<=A.t` compare with NaN `A.t` fails | same sentinel as C; must match bit-exactly |
| 5  | `c2RaytoCircle` | `disc` NaN (any NaN input) → `disc < 0` false, `sqrtf(NaN)=NaN`, `t=NaN`, `NaN>=0` false | returns `0` |
| 6  | `c2AABBtoAABB` (L130-134) | `B.max.x < A.min.x` (separated on -x) | returns `0` |
| 7  | `c2AABBtoAABB` (L131,134) | `A.max.x < B.min.x` (separated on +x) | returns `0` |
| 8  | `c2AABBtoAABB` (L132,134) | `B.max.y < A.min.y` (separated on -y) | returns `0` |
| 9  | `c2AABBtoAABB` (L133,134) | `A.max.y < B.min.y` (separated on +y) | returns `0` |
| 10 | `c2AABBtoAABB` | any NaN coordinate → all four `<` are false → `!(0)` | returns `1` (NaN reported as overlapping) |
| 11 | `c2RayToPlane_OneDimensional` (L143-144) | `da < 0` — start point already on the far side of the plane | returns `0.0f` |
| 12 | `c2RayToPlane_OneDimensional` (L145-146) | `da*db > 0` — both endpoints on the same side | returns `1.0f` |
| 13 | `c2RayToPlane_OneDimensional` (L149-152) | `d = da-db == 0` — degenerate (ray parallel / zero length in that axis) | returns `0.0f` |
| 14 | `c2RaytoAABB` (L162-163) | ray's own AABB does not overlap `B` (`!c2AABBtoAABB`) | returns `0`, `*out` untouched |
| 15 | `c2RaytoAABB` (L173-174) | separating-axis test `d > 0` (ray line misses the box) | returns `0`, `*out` untouched |
| 16 | `c2RaytoAABB` (L211-212) | `hit0|hit1|hit2|hit3 == 0`, i.e. every `t_i > 1.0f` | returns `0`, `*out` untouched |
| 17 | `c2RaytoAABB` | inverted box (`B.min > B.max`) — no validation; `half_extents` negative | whatever the arithmetic yields; must match |
| 18 | `c2RaytoAABB` | `A.t == 0` → `p1 == p0`, `ab` zero, `n` zero, `d = 0 - dot(0,he)` | must match C exactly |
| 19 | `c2AABBtoPoint` (L235-239) | `B.x < A.min.x` | returns `0` |
| 20 | `c2AABBtoPoint` (L236,239) | `B.y < A.min.y` | returns `0` |
| 21 | `c2AABBtoPoint` (L237,239) | `B.x > A.max.x` | returns `0` |
| 22 | `c2AABBtoPoint` (L238,239) | `B.y > A.max.y` | returns `0` |
| 23 | `c2AABBtoPoint` | any NaN → all four comparisons false | returns `1` |
| 24 | `c2CircleToPoint` (L242-246) | `dot(n,n) >= A.r*A.r` (point on/outside the circle; note **strict** `<`, so a point exactly on the rim is rejected) | returns `0` |
| 25 | `c2CircleToPoint` | `A.r == 0` → `d2 < 0` impossible | always returns `0` |
| 26 | `c2CircleToPoint` | `A.r < 0` → `r*r` positive, so negative radius behaves as positive | returns `1` for inside points |
| 27 | `c2RaytoCapsule` (L248-308) | `B.a == B.b` (zero-length capsule) → `c2Norm(0)` divides by 0 → `M.y = (NaN,NaN)`, so `yBb` and `yAp` are NaN too. **Corrected by testing:** every `<`/`>` in `c2AABBtoPoint` is then false, so the C reports the origin as INSIDE the local bb and returns **1** — not 0 as first assumed | returns `1`, `*out = { t: 0, n: (NaN, NaN) }` |
| 28 | `c2RaytoCapsule` (L307-308) | falls through: `yAe.x*yAp.x >= 0` **and** `min(|yAe.x|,|yAp.x|) >= B.r` | returns `0`, but `*out` **was already overwritten** with `n=norm(cap_n)`, `t=0` |
| 29 | `c2RaytoCapsule` (L289,291,298,300) | delegates to `c2RaytoCircle`, which itself rejects (rows 1-3) | propagates `0` from `c2RaytoCircle`; `*out` has the pre-set `n`/`t=0` |
| 30 | `c2RaytoCapsule` (L294-296) | `d = yAe.x - yAp.x == 0` → `t = (c-yAp.x)/0` → `±inf` or NaN | must match C |
| 31 | `c2RaytoCapsule` | `B.r < 0` → `capsule_bb.min.x = -B.r > 0 > B.r = max.x` (inverted bb) → `c2AABBtoPoint` fails | must match C |
| 32 | `c2RaytoPoly` (L347-348) | `den == 0 && num < 0` — ray parallel to and outside an edge plane | returns `0`, `*out` untouched |
| 33 | `c2RaytoPoly` (L356-357) | `hi < lo` — the accumulated slab interval became empty | returns `0`, `*out` untouched |
| 34 | `c2RaytoPoly` (L361-364) | loop completes with `index == ~0` (no `den<0 && num<lo*den` ever fired) | returns `0`, `*out` untouched |
| 35 | `c2RaytoPoly` (L340) | `B->count == 0` — loop body never runs, `index` stays `~0` | returns `0`, `*out` untouched |
| 36 | `c2RaytoPoly` (L340) | `B->count < 0` (e.g. `-1`, `INT_MIN`) — `i < count` false immediately | returns `0`, `*out` untouched |
| 37 | `c2RaytoPoly` (L336) | `bx_ptr == NULL` — the library's only null check; substitutes `c2xIdentity()` | must NOT crash; identical result to passing an explicit identity `c2x` |
| 38 | `c2RaytoPoly` | `A.t == 0` → `hi = 0`; any `den>0 && num<0` shrinks `hi` below `lo=0` | returns `0` |
| 39 | `c2RaytoPoly` | `A.t < 0` → `hi < lo == 0` on the first iteration | returns `0` |
| 40 | `c2RaytoPoly` | `bx.r` non-normalised / zero (`c=0,s=0`) — no validation | must match C |
| 41 | `c2CastRay` (L367-378) | `typeB` is an **out-of-range enum value** (`4`, `5`, `-1`, `INT_MIN`, `INT_MAX`) — C enums accept any `int`, the `switch` has no `default`, so control reaches the trailing `return 0` | returns `0`, `*out` untouched, `B` never dereferenced |
| 42 | `c2CastRay` | `typeB == C2_TYPE_POLY` with `bx == NULL` | delegates to row 37 |
| 43 | `c2CastRay` | `typeB` valid but the shape behind `B` triggers any of rows 1-40 | propagates the delegate's sentinel unchanged |
| 44 | `c2Div` / `c2Norm` | `b == 0` / `c2Len(a) == 0` → `1.0f/0.0f = +inf`, `0*inf = NaN` | `(NaN,NaN)`; sign/bit pattern must match |
| 45 | `c2Norm` | `a = (-0.0,-0.0)` → `len = 0`, `1/0 = inf`, `-0*inf = NaN` | must match C bit-exactly |
| 46 | `c2Len` | `c2Dot(a,a) < 0` impossible for finite, but `inf-inf` → NaN → `sqrtf(NaN)` | `NaN` |
| 47 | `c2Len` | any component `±inf` → `inf` | `+inf` |
| 48 | `c2Absv` / `c_abs` | input `-0.0f`: C uses `x < 0 ? -x : x`, and `-0.0 < 0` is **false**, so `-0.0` is returned **unchanged** (NOT `+0.0` as `fabsf` would give) | `-0.0f` preserved; bit pattern `0x80000000` |
| 49 | `c2Minv`/`c2Maxv` | NaN operand: C's ternary `a<b?a:b` returns **`b`** when the compare is false, unlike `fminf` | NaN-propagation pattern must match exactly |
| 50 | all `*out` writers | `out == NULL` | **no null check anywhere** — C dereferences and segfaults. Not exercised (identical UB in both); documented, deliberately untested. |

## Coverage note

Rows 1-49 all have differential tests in `translation/tests/phase_c_errors.rs`
(plus the exhaustive sweeps in `tests/nan_storm.rs`). Row 50 is
UB-by-construction in the C (a raw unchecked `out->t = ...`), so a
"differential" test would only compare two segfaults; it is documented rather
than executed.

Two rows were **corrected by testing** rather than left as written, because the
C's actual behaviour contradicted the first reading of the source:

- Row 27 (`B.a == B.b`) returns **1**, not 0: the NaN local frame makes every
  comparison in `c2AABBtoPoint` false, which that function reports as "inside".
- An oversized `c2Poly.count` does **not** always bail out early; it only does
  so when an early edge triggers rows 32/33. `tests/phase_c_errors.rs`
  therefore rigs edge 0 to reject before testing `count = INT_MAX`.

The `expect_c(...)` helper asserts the C really produced the documented
sentinel, so a row cannot pass by never reaching its trigger. Rows whose
trigger is only reachable probabilistically (15, 28, 29, 32, 33) additionally
count occurrences and assert the count is non-zero.
