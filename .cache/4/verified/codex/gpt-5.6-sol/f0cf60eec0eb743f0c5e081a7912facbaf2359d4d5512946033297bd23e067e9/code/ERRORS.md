# Error Surface

This table is derived from every C branch that rejects an input by returning
zero. A zero return is a geometric miss/rejection sentinel, not an errno-style
failure. Rows split compound conditions where distinct boundary inputs reach
the same return statement.

The C source contains no `assert`, `RETURN_ERROR`, `return -1`, `return NULL`,
or explicit pointer validation. Null output pointers are only safe on paths
that do not write output. A null shape pointer, a null output pointer on a hit,
or `c2Poly.count > 8` invokes C undefined behavior and therefore has no defined
C result to place in this table.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `c2RaytoCircle` | `disc = b*b - c < 0` | returns `0`; `out` unchanged | [x] |
| 2 | `c2RaytoCircle` | real intersection root `t < 0` | returns `0`; `out` unchanged | [x] |
| 3 | `c2RaytoCircle` | real intersection root `t > A.t` | returns `0`; `out` unchanged | [x] |
| 4 | `c2AABBtoAABB` | `B.max.x < A.min.x` | returns `0` | [x] |
| 5 | `c2AABBtoAABB` | `A.max.x < B.min.x` | returns `0` | [x] |
| 6 | `c2AABBtoAABB` | `B.max.y < A.min.y` | returns `0` | [x] |
| 7 | `c2AABBtoAABB` | `A.max.y < B.min.y` | returns `0` | [x] |
| 8 | `c2RaytoAABB` | ray segment AABB does not overlap `B` | returns `0`; `out` unchanged | [x] |
| 9 | `c2RaytoAABB` | overlap passes, but separating-axis distance `d > 0` | returns `0`; `out` unchanged | [x] |
| 10 | `c2RaytoAABB` | all four plane-hit checks `t0..t3 <= 1` are false | returns `0`; `out` unchanged | [x] |
| 11 | `c2AABBtoPoint` | `B.x < A.min.x` | returns `0` | [x] |
| 12 | `c2AABBtoPoint` | `B.y < A.min.y` | returns `0` | [x] |
| 13 | `c2AABBtoPoint` | `B.x > A.max.x` | returns `0` | [x] |
| 14 | `c2AABBtoPoint` | `B.y > A.max.y` | returns `0` | [x] |
| 15 | `c2CircleToPoint` | squared distance `d2 >= A.r * A.r` (including boundary equality) | returns `0` | [x] |
| 16 | `c2RaytoCapsule` | start is outside body/endcaps and segment fails `yAe.x*yAp.x < 0 || min(abs(yAe.x), abs(yAp.x)) < B.r` | returns `0`; initial normal and `t = 0` remain in `out` | [x] |
| 17 | `c2RaytoPoly` | for an edge, `den == 0 && num < 0` | returns `0`; `out` unchanged | [x] |
| 18 | `c2RaytoPoly` | edge clipping produces `hi < lo` | returns `0`; `out` unchanged | [x] |
| 19 | `c2RaytoPoly` | loop completes with `index == ~0` | returns `0`; `out` unchanged | [x] |
| 20 | `c2RaytoPoly` | `B.count == 0`, so no entering edge can set `index` | returns `0`; `out` unchanged | [x] |
| 21 | `c2RaytoPoly` | `B.count < 0`, so the loop is skipped | returns `0`; `out` unchanged | [x] |
| 22 | `c2CastRay` | `typeB` is outside enum values `0..=3` | returns `0`; does not dereference `B`, `bx`, or `out` | [x] |

## Generic FFI Boundaries

- [x] Null transform pointers are compared on polygon calls.
- [x] Null output pointers are compared on non-writing miss paths.
- [x] Null shape/output/transform pointers are compared for invalid dispatch
  enums, where C does not dereference them.
- [x] Zero and negative polygon counts are compared.
- [x] Oversized counts `9` and `INT_MAX` are compared on a branch that rejects
  before indexing beyond the fixed arrays.
- [x] Enum values `-1` and `4`, plus `INT_MIN`, `INT_MAX`, and randomized
  out-of-range values are compared.

Null pointers on writing paths and oversized counts that continue beyond index
7 are not rejection cases: they invoke C undefined behavior and have no stable
C result to compare.
