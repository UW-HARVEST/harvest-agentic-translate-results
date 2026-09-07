# Error and rejection surface

The C API has no error enum and no function returns `-1` or `NULL` as a
documented failure sentinel. Rejections are represented by zero contacts,
early return, ignored unsupported enum values, or optional output pointers.
Rows below are taken from explicit C branches and required FFI boundaries.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `c2MakeProxy` | `type` is `C2_TYPE_POLY` or any value outside capsule/circle/AABB | No `switch` arm runs; caller-provided proxy bytes are unchanged. [x] |
| 2 | `c2GJKSimplexMetric` | `s->count` is not 2 or 3, including 0, 1, negative, or greater than 3 | Returns `0.0f`. [x] |
| 3 | `c2D` | `s->count` is 3 or any value outside 1/2 | Returns `(0.0f, 0.0f)`. [x] |
| 4 | `c2Witness` | `s->count` is outside 1/2/3 | Writes `(0,0)` to both output vectors. [x] |
| 5 | `c2L` | `s->count` is outside 1/2 | Returns `(0,0)`. [x] |
| 6 | `c2Collide` | `typeA` is `C2_TYPE_POLY` or out of range | Sets `m->count = 0`; does not dereference either shape. [x] |
| 7 | `c2Collide` | `typeA` is supported but `typeB` is `C2_TYPE_POLY` or out of range | Sets `m->count = 0`; no nested arm runs. [x] |
| 8 | `omni_manifold` | either type is unsupported/out of range | `c2Collide` leaves `m->count = 0`; `ptr_from_parts` itself has undefined return value for that type. [x] |
| 9 | `c2CircletoCircleManifold` | squared center distance is greater than or equal to `(A.r + B.r)^2` | Sets `m->count = 0`; tangent circles are rejected because the comparison is strict. [x] |
| 10 | `c2CircletoAABBManifold` | squared distance from center to clamped point is greater than or equal to `A.r^2` | Sets `m->count = 0`; tangency is rejected. [x] |
| 11 | `c2CircletoCapsuleManifold` | GJK centerline distance is greater than or equal to `A.r + B.r` | Sets `m->count = 0`; tangency is rejected. [x] |
| 12 | `c2AABBtoAABBManifold` | x-axis overlap `dx < 0` | Sets count to zero then returns. [x] |
| 13 | `c2AABBtoAABBManifold` | `dx >= 0` and y-axis overlap `dy < 0` | Sets count to zero then returns. [x] |
| 14 | `c2CapsuletoPolyManifold` | GJK distance is at least both `1.0e-6f` and `A.r` | Leaves `m->count = 0`. [x] |
| 15 | `c2CapsuletoPolyManifold` | code 0 clipping: first or second side plane leaves fewer than two points | Returns early with the manifold state produced so far (normally count zero). [x] |
| 16 | `c2CapsuletoPolyManifold` | code 1 clipping leaves fewer than two incident points | Returns early with count zero. [x] |
| 17 | `c2CapsuletoPolyManifold` | code 2 clipping leaves fewer than two incident points | Returns early with count zero. [x] |
| 18 | `c2CapsuletoCapsuleManifold` | GJK centerline distance is greater than or equal to `A.r + B.r` | Sets `m->count = 0`; tangency is rejected. [x] |
| 19 | `c2GJK` | `ax_ptr == NULL` | Uses identity transform for A. [x] |
| 20 | `c2GJK` | `bx_ptr == NULL` | Uses identity transform for B. [x] |
| 21 | `c2GJK` | `outA == NULL`, `outB == NULL`, or `iterations == NULL` | Skips only the corresponding optional write and still returns distance. [x] |
| 22 | `c2GJK` | `cache == NULL` | Starts from vertex zero and does not read/write a cache. [x] |
| 23 | `c2GJK` | cache is supplied with `cache->count == 0` | Treats cache as empty and starts from vertex zero. [x] |
| 24 | `c2GJK` | cached simplex fails the source metric predicate | Ignores the cached simplex and starts from vertex zero. [x] |
| 25 | `c2GJK` | search direction squared is below `FLT_EPSILON^2`, support pair duplicates a saved pair, distance increases, simplex reaches 3, or 20 iterations complete | Terminates iteration and returns the current witness distance. [x] |
| 26 | `c2Support` | `count == 1` | Returns index 0. [x] |
| 27 | `c2Norms` | `count == 0` | Performs no writes. [x] |
| 28 | required-pointer APIs | a required pointer (`m`, `shape`, `p`, `verts` with positive count, `s`, `B`, etc.) is NULL | C performs a null dereference: undefined behavior, normally process termination; there is no error sentinel. [x] |
| 29 | fixed-array/count APIs | counts or indices exceed backing arrays (`c2Poly` > 8, simplex/cache > 3, negative/out-of-range index) | C performs out-of-bounds access: undefined behavior; there is no defined rejection value. [x] |
| 30 | `c2Div`, `c2Norm`, `c2Intersect` | zero divisor, zero-length vector, or `da == db` | IEEE-754 division by zero produces infinities/NaNs; no explicit rejection. [x] |
| 31 | `ptr_from_parts` | valid shape type but `malloc` returns NULL | C dereferences NULL: undefined behavior. [x] |
| 32 | `ptr_from_parts` | unsupported/out-of-range enum | Control reaches the end of a non-void function: undefined/indeterminate pointer; no defined sentinel. [x] |
| 33 | `c2GJK` via `c2CapsuletoPolyManifold` / `c2AABBtoCapsuleManifold` | `typeB == C2_TYPE_POLY`; `c2MakeProxy` has no polygon arm, so the local polygon proxy remains uninitialized | Subsequent count/vertex reads are undefined and stack-history-dependent. Stable no-contact calls are differentially tested; overlap/contact outputs have no deterministic C value. [x] |
