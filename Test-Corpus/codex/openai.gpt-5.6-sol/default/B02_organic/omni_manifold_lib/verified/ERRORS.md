# Error surface

The C source contains no `assert`, `RETURN_ERROR`, error enum, `return -1`, or
explicit error code. Rejection is represented by a zero-contact manifold, or
by leaving output unchanged for unsupported enum/count values. Each row below
corresponds to a distinct explicit rejection branch in `src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E01 [x] | `c2CircletoCircleManifold` | `dot(B.p-A.p, B.p-A.p) >= (A.r+B.r)^2` (including exact tangency) | `m->count = 0` |
| E02 [x] | `c2CircletoAABBManifold` | squared distance from circle center to clamped AABB point is `>= A.r^2` | `m->count = 0` |
| E03 [x] | `c2CircletoCapsuleManifold` | GJK centerline distance is `>= A.r+B.r` | `m->count = 0` |
| E04 [x] | `c2AABBtoAABBManifold` | `eA.x + eB.x - abs(mid_b.x-mid_a.x) < 0` | `m->count = 0`; return before the y test |
| E05 [x] | `c2AABBtoAABBManifold` | x test passes and `eA.y + eB.y - abs(mid_b.y-mid_a.y) < 0` | `m->count = 0` |
| E07 [x] | `c2AABBtoCapsuleManifold` | delegated capsule/poly test produces no contact | `m->count = 0` |
| E08 [x] | `c2CapsuletoCapsuleManifold` | GJK centerline distance is `>= A.r+B.r` | `m->count = 0` |
| E09 [x] | `c2Collide` | `typeA` is outside `C2_TYPE_CAPSULE..=C2_TYPE_POLY` | initialize `m->count = 0`, take no case, return |
| E10 [x] | `c2Collide` | `typeA` is circle/AABB/capsule and `typeB` has no matching inner-switch case (poly or out-of-range) | initialize `m->count = 0`, take no inner case, return |
| E11 [x] | `c2MakeProxy` | `type` is poly or outside the enum range | take no switch case; proxy bytes remain unchanged |
| E12 [x] | `c2Norms` | `count == 0` | loop executes zero times; output remains unchanged |
| E13 [x] | `c2Norms` | `count < 0` | loop executes zero times; output remains unchanged |
| E14 [x] | `c2Support` | `count == 0` with a valid first vertex pointer | reads vertex zero, loop executes zero times, returns index `0` |
| E15 [x] | `c2Norms` | `count == 9` (one past internal polygon capacity) with nine valid input/output elements | no maximum check; processes all nine elements |
| E16 [x] | `c2Support` | `count == 9` with nine valid vertices | no maximum check; searches all nine elements |

## Undefined contracts

The C API does not reject null required pointers or oversized backing arrays.
Dereferencing a null required pointer, indexing beyond the caller's allocation,
using an invalid shape enum in `c2GJK`/`omni_manifold`, requesting a poly proxy
(the poly enum has no `c2MakeProxy` switch case), or passing a polygon count
outside the backing arrays invokes C undefined behavior and has no C result
that Rust can validly match. `ptr_from_parts` also reaches the end of a
non-void function without returning for poly or out-of-range enum values.
Optional null pointers explicitly handled by defined `c2GJK` shape modes
(transforms/outputs/iterations/cache) are valid configurations in
`CONFIGS.md`; the capsule/poly transform pointer is handled, but the enclosing
call still reaches the uninitialized poly proxy.
