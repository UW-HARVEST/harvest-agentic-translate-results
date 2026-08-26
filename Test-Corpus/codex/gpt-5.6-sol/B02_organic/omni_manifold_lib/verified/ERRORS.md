# Error Surface

The C source has no `assert`, `RETURN_ERROR`, error enum, `return -1`, or
explicit `return NULL`. It reports geometric rejection by leaving
`c2Manifold.count` at zero, and reports unsupported type dispatch by doing
nothing. These are all defined rejection branches found in the source.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| E01 | `c2MakeProxy` | `type` is `C2_TYPE_POLY` or any value outside the three handled switch cases | output proxy remains byte-for-byte unchanged | [x] |
| E02 | `c2CircletoCircleManifold` | `dot(B.p-A.p, B.p-A.p) >= (A.r+B.r)^2` | `m->count = 0`; remaining manifold bytes unchanged | [x] |
| E03 | `c2CircletoAABBManifold` | squared distance from circle center to clamped AABB point is `>= A.r^2` | `m->count = 0`; remaining manifold bytes unchanged | [x] |
| E04 | `c2CircletoCapsuleManifold` | GJK distance is `>= A.r+B.r` | `m->count = 0`; remaining manifold bytes unchanged | [x] |
| E05 | `c2AABBtoAABBManifold` | x overlap `dx < 0` | `m->count = 0`; immediate return | [x] |
| E06 | `c2AABBtoAABBManifold` | x overlap is nonnegative and y overlap `dy < 0` | `m->count = 0`; immediate return | [x] |
| E07 | `c2CapsuletoPolyManifold` | `d < 1e-6`, reference-face path selected, and first/second side-plane clipping yields fewer than two points | immediate return with the manifold state produced so far | N/A: C UB |
| E08 | `c2CapsuletoPolyManifold` | `d < 1e-6`, first capsule-face path selected, and side-plane clipping yields fewer than two points | immediate return with the manifold state produced so far | N/A: C UB |
| E09 | `c2CapsuletoPolyManifold` | `d < 1e-6`, second capsule-face path selected, and side-plane clipping yields fewer than two points | immediate return with the manifold state produced so far | N/A: C UB |
| E10 | `c2CapsuletoPolyManifold` | GJK distance `d >= A.r` | `m->count = 0`; no contact | N/A: C UB |
| E11 | `c2CapsuletoCapsuleManifold` | GJK distance is `>= A.r+B.r` | `m->count = 0`; remaining manifold bytes unchanged | [x] |
| E12 | `c2Collide` | valid outer `typeA`, but `typeB` is `C2_TYPE_POLY` or an out-of-range integer | `m->count = 0`; no inner switch case runs | [x] |
| E13 | `c2Collide` | `typeA` is `C2_TYPE_POLY` or an out-of-range integer | `m->count = 0`; no outer switch case runs | [x] |
| E14 | `omni_manifold` | either dispatch type has no `c2Collide` case (`C2_TYPE_POLY` or out-of-range), while execution reaches `c2Collide` | nominally `m->count = 0`, but `ptr_from_parts` first falls off a non-void function | N/A: C UB |

## FFI Boundary Limits

The public C functions do not validate null pointers, zero counts, oversized
counts, invalid polygon counts, or out-of-range indices. Passing those values
to a path that dereferences/indexes them has undefined behavior, so C defines
no error code or sentinel to compare. Differential tests cover null optional
pointers where the C source explicitly checks them (`c2GJK` transforms,
outputs, iterations, and cache; `c2CapsuletoPolyManifold` transform), zero
iteration counts where no dereference occurs (`c2Norms`), and invalid enums on
the defined no-dispatch paths above. `ptr_from_parts` itself falls off the end
for unsupported enum values, which is also undefined behavior rather than a
defined rejection.

`c2CapsuletoPolyManifold` passes `C2_TYPE_POLY` to `c2GJK`, but
`c2MakeProxy` has no polygon arm, leaving the local proxy uninitialized.
MemorySanitizer reports the resulting use of uninitialized data at
`c_src/src/lib.c:506`, called from `c_src/src/lib.c:733`. The fresh default C
build produced both divergent bytes and SIGSEGV across valid randomized
polygon calls, so E07-E10 have no deterministic C result to assert.
