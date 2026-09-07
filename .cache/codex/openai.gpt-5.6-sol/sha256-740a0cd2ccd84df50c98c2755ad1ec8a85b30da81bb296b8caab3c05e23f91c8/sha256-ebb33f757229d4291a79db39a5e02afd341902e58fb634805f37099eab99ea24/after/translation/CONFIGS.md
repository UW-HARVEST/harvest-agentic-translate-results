# Configuration surface

Derived mechanically from all 10 exported C entry points, the strict
comparisons in `c2Maxv`, `c2Minv`, the composed clamp branches, the collision
comparisons, the four AABB separation comparisons, and the nested dispatch
switches in `collided`.

For vector relation rows, `G/L/E/U` mean greater-than, less-than, equal, and
unordered (NaN). For circle/AABB rows, the point region is relative to the
box. For AABB/AABB rows, B's interval relation is relative to A on each axis:
`S-` is separated on the negative side, `T-` touches the negative side, and
`O` overlaps; reflections cover the corresponding positive-side cases.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | arbitrary finite `x`, `y` (including signed zero) | [x] |
| 2 | `c2V` | IEEE non-finite values and NaN payloads | [x] |
| 3 | `c2Maxv` | `(a.x > b.x, a.y > b.y) = (true, true)` | [x] |
| 4 | `c2Maxv` | comparison outcomes `(true, false)` | [x] |
| 5 | `c2Maxv` | comparison outcomes `(false, true)` | [x] |
| 6 | `c2Maxv` | comparison outcomes `(false, false)` | [x] |
| 7 | `c2Maxv` | equality and unordered operands exercise the false arm and preserve B's selected bits | [x] |
| 8 | `c2Minv` | `(a.x < b.x, a.y < b.y) = (true, true)` | [x] |
| 9 | `c2Minv` | comparison outcomes `(true, false)` | [x] |
| 10 | `c2Minv` | comparison outcomes `(false, true)` | [x] |
| 11 | `c2Minv` | comparison outcomes `(false, false)` | [x] |
| 12 | `c2Minv` | equality and unordered operands exercise the false arm and preserve B's selected bits | [x] |
| 13 | `c2Clampv` | x below `[lo,hi]`, y below `[lo,hi]` | [x] |
| 14 | `c2Clampv` | x below, y within | [x] |
| 15 | `c2Clampv` | x below, y above | [x] |
| 16 | `c2Clampv` | x within, y below | [x] |
| 17 | `c2Clampv` | x within, y within | [x] |
| 18 | `c2Clampv` | x within, y above | [x] |
| 19 | `c2Clampv` | x above, y below | [x] |
| 20 | `c2Clampv` | x above, y within | [x] |
| 21 | `c2Clampv` | x above, y above | [x] |
| 22 | `c2Clampv` | equality at each bound and signed-zero boundaries | [x] |
| 23 | `c2Clampv` | unordered operands and inverted bounds, which strict comparisons handle without validation | [x] |
| 24 | `c2Sub` | arbitrary finite vectors, including cancellation and signed zero | [x] |
| 25 | `c2Sub` | infinities and NaNs | [x] |
| 26 | `c2Dot` | arbitrary finite vectors with positive, negative, zero, cancellation, overflow, and underflow products | [x] |
| 27 | `c2Dot` | infinities and NaNs | [x] |
| 28 | `c2CircletoCircle` | `distance² < (rA+rB)²` (overlap) | [x] |
| 29 | `c2CircletoCircle` | `distance² == (rA+rB)²` (tangent; strict comparison returns `0`) | [x] |
| 30 | `c2CircletoCircle` | `distance² > (rA+rB)²` (separated) | [x] |
| 31 | `c2CircletoCircle` | zero and negative radii, used exactly as C squares their sum | [x] |
| 32 | `c2CircletoCircle` | unordered/non-finite values make the final strict comparison false when unordered | [x] |
| 33 | `c2CircletoAABB` | point region left-below; final result overlap | [x] |
| 34 | `c2CircletoAABB` | point region left-below; tangent | [x] |
| 35 | `c2CircletoAABB` | point region left-below; separated | [x] |
| 36 | `c2CircletoAABB` | point region left-within-y; overlap | [x] |
| 37 | `c2CircletoAABB` | point region left-within-y; tangent | [x] |
| 38 | `c2CircletoAABB` | point region left-within-y; separated | [x] |
| 39 | `c2CircletoAABB` | point region left-above; overlap | [x] |
| 40 | `c2CircletoAABB` | point region left-above; tangent | [x] |
| 41 | `c2CircletoAABB` | point region left-above; separated | [x] |
| 42 | `c2CircletoAABB` | point region within-x/below; overlap | [x] |
| 43 | `c2CircletoAABB` | point region within-x/below; tangent | [x] |
| 44 | `c2CircletoAABB` | point region within-x/below; separated | [x] |
| 45 | `c2CircletoAABB` | point inside/on box in both axes; positive-radius overlap | [x] |
| 46 | `c2CircletoAABB` | point inside/on box with zero radius; strict comparison returns `0` | [x] |
| 47 | `c2CircletoAABB` | point region within-x/above; overlap | [x] |
| 48 | `c2CircletoAABB` | point region within-x/above; tangent | [x] |
| 49 | `c2CircletoAABB` | point region within-x/above; separated | [x] |
| 50 | `c2CircletoAABB` | point region right-below; overlap | [x] |
| 51 | `c2CircletoAABB` | point region right-below; tangent | [x] |
| 52 | `c2CircletoAABB` | point region right-below; separated | [x] |
| 53 | `c2CircletoAABB` | point region right-within-y; overlap | [x] |
| 54 | `c2CircletoAABB` | point region right-within-y; tangent | [x] |
| 55 | `c2CircletoAABB` | point region right-within-y; separated | [x] |
| 56 | `c2CircletoAABB` | point region right-above; overlap | [x] |
| 57 | `c2CircletoAABB` | point region right-above; tangent | [x] |
| 58 | `c2CircletoAABB` | point region right-above; separated | [x] |
| 59 | `c2CircletoAABB` | inverted box bounds and unordered/non-finite values, with no validation | [x] |
| 60 | `c2AABBtoAABB` | x=`S-`, y=`S-` (diagonally separated) | [x] |
| 61 | `c2AABBtoAABB` | x=`S-`, y=`T-` | [x] |
| 62 | `c2AABBtoAABB` | x=`S-`, y=`O` | [x] |
| 63 | `c2AABBtoAABB` | x=`T-`, y=`S-` | [x] |
| 64 | `c2AABBtoAABB` | x=`T-`, y=`T-` (corner touch counts as collision) | [x] |
| 65 | `c2AABBtoAABB` | x=`T-`, y=`O` | [x] |
| 66 | `c2AABBtoAABB` | x=`O`, y=`S-` | [x] |
| 67 | `c2AABBtoAABB` | x=`O`, y=`T-` | [x] |
| 68 | `c2AABBtoAABB` | x=`O`, y=`O` (including containment/equality) | [x] |
| 69 | `c2AABBtoAABB` | positive-side reflections of rows 60-68 | [x] |
| 70 | `c2AABBtoAABB` | inverted bounds and unordered/non-finite endpoints, with no validation | [x] |
| 71 | `collided` | `typeA=CIRCLE`, `typeB=CIRCLE`; pointers carry circles | [x] |
| 72 | `collided` | `typeA=CIRCLE`, `typeB=AABB`; pointers carry circle/AABB | [x] |
| 73 | `collided` | `typeA=AABB`, `typeB=CIRCLE`; reversed dispatch swaps arguments into circle/AABB order | [x] |
| 74 | `collided` | `typeA=AABB`, `typeB=AABB`; pointers carry AABBs | [x] |

Feature surface: `Cargo.toml` declares no features, so the sole feature
combination is the default/no-feature build.
