# Error And Rejection Surface

The C API has no error enum, error macro, assertion, explicit null check,
length argument, or documented numeric range. A return value of zero from the
geometry routines means "no hit" or "not contained", rather than a separate
API error. The rows below mechanically enumerate every distinct source
condition that rejects a ray operation.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E01 | `c2RaytoCircle` | `disc = dot(m,m) - r*r` after subtracting `b*b`, then `disc < 0` | `0`; `out` unchanged |
| E02 | `c2RaytoCircle` | `disc >= 0`, but `t = -b - sqrtf(disc)` is `< 0` | `0`; `out` unchanged |
| E03 | `c2RaytoCircle` | `disc >= 0`, but computed `t > A.t` | `0`; `out` unchanged |
| E04 | `c2RaytoAABB` | ray-segment bounding box and target fail `c2AABBtoAABB` | `0`; `out` unchanged |
| E05 | `c2RaytoAABB` | bounding boxes overlap, but separating-axis value `d > 0` | `0`; `out` unchanged |
| E06 | `c2RaytoAABB` | all four one-dimensional plane parameters compare `> 1.0f` | `0`; `out` unchanged |
| E07 | `c2RaytoCapsule` | start is outside body and caps, and neither `yAe.x*yAp.x < 0` nor `min(abs(yAe.x),abs(yAp.x)) < B.r` | `0`; unlike the other ray routines, `out` is first set to `{0, norm(B.b-B.a)}` |
| E08 | `c2CastRay` | `typeB` is not `0`, `1`, or `2` | the source reaches the end of a non-void function (undefined by C); this x86-64 C `.so` returns the incoming `EAX` value and leaves `out` unchanged |
| E09 | `c2RaytoCircle` | `out == NULL` and the ray misses (E01-E03) | `0`; pointer is not dereferenced |
| E10 | `c2RaytoAABB` | `out == NULL` and the ray misses (E04-E06) | `0`; pointer is not dereferenced |
| E11 | `c2CastRay` | `B == NULL` with out-of-range `typeB` | same observed result as E08; `B` and `out` are not dereferenced |
| E12 | `spec_ray` | `cast == NULL` and the generated circle ray misses | `0`; pointer is not dereferenced |

The following pointer cases are not C rejections and have no return value to
compare: a null `out` on a successful `c2RaytoCircle`/`c2RaytoAABB`, any null
`out` passed to `c2RaytoCapsule`, a null valid-type shape passed to
`c2CastRay`, and a null `cast` on a successful `spec_ray` call all perform a
null dereference. They are exercised in isolated subprocesses so a signal
cannot terminate the differential test process.

There are no zero/oversized length cases because no exported function accepts
a length. Numeric zero, infinities, NaNs, negative radii, degenerate shapes,
and one-past-range enum values are covered as FFI boundary configurations.

- [x] E01
- [x] E02
- [x] E03
- [x] E04
- [x] E05
- [x] E06
- [x] E07
- [x] E08
- [x] E09
- [x] E10
- [x] E11
- [x] E12
