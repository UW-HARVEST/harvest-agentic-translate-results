# Configuration Surface

The public surface is the complete `nm -D` function set, including helpers not
declared in `include/lib.h`. Rows mechanically enumerate the outcomes of every
comparison and switch branch in `src/lib.c`. Each row is exercised with fixed-
seed randomized finite inputs plus applicable zero, equality, infinity, and
NaN cases.

`LT`, `EQ`, and `GT` below describe the exact floating-point comparison shown.
`UNORD` means at least one operand is NaN, so the C comparison is false.
For clamp axes, the branch states are:

- `below`: `a < hi` and `lo > a`
- `inside`: `a < hi` and not `lo > a`
- `inverted`: not `a < hi` and `lo > hi`
- `above`: not `a < hi` and not `lo > hi`

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | arbitrary `x` and `y` bit patterns, returned as a two-float struct | [x] |
| 2 | `c2Maxv` | x comparison true (`a.x > b.x`), y comparison true | [x] |
| 3 | `c2Maxv` | x comparison true, y comparison false (`LT`, `EQ`, or `UNORD`) | [x] |
| 4 | `c2Maxv` | x comparison false, y comparison true | [x] |
| 5 | `c2Maxv` | x comparison false, y comparison false | [x] |
| 6 | `c2Minv` | x comparison true (`a.x < b.x`), y comparison true | [x] |
| 7 | `c2Minv` | x comparison true, y comparison false (`GT`, `EQ`, or `UNORD`) | [x] |
| 8 | `c2Minv` | x comparison false, y comparison true | [x] |
| 9 | `c2Minv` | x comparison false, y comparison false | [x] |
| 10 | `c2Clampv` | x `below`, y `below` | [x] |
| 11 | `c2Clampv` | x `below`, y `inside` | [x] |
| 12 | `c2Clampv` | x `below`, y `inverted` | [x] |
| 13 | `c2Clampv` | x `below`, y `above` | [x] |
| 14 | `c2Clampv` | x `inside`, y `below` | [x] |
| 15 | `c2Clampv` | x `inside`, y `inside` | [x] |
| 16 | `c2Clampv` | x `inside`, y `inverted` | [x] |
| 17 | `c2Clampv` | x `inside`, y `above` | [x] |
| 18 | `c2Clampv` | x `inverted`, y `below` | [x] |
| 19 | `c2Clampv` | x `inverted`, y `inside` | [x] |
| 20 | `c2Clampv` | x `inverted`, y `inverted` | [x] |
| 21 | `c2Clampv` | x `inverted`, y `above` | [x] |
| 22 | `c2Clampv` | x `above`, y `below` | [x] |
| 23 | `c2Clampv` | x `above`, y `inside` | [x] |
| 24 | `c2Clampv` | x `above`, y `inverted` | [x] |
| 25 | `c2Clampv` | x `above`, y `above` | [x] |
| 26 | `c2Sub` | arbitrary two-vector subtraction, including cancellation and non-finite operands | [x] |
| 27 | `c2Dot` | arbitrary two-vector dot product, including zero, overflow, cancellation, and non-finite operands | [x] |
| 28 | `c2CircletoCircle` | squared center distance `<` squared radius sum | [x] |
| 29 | `c2CircletoCircle` | squared center distance `==` squared radius sum (strict tangent boundary) | [x] |
| 30 | `c2CircletoCircle` | squared center distance `>` squared radius sum, or comparison `UNORD` | [x] |
| 31 | `c2CircletoAABB` | center x `below`, y `below`; distance squared `<` radius squared | [x] |
| 32 | `c2CircletoAABB` | center x `below`, y `below`; distance squared `==` radius squared | [x] |
| 33 | `c2CircletoAABB` | center x `below`, y `below`; distance squared `>` radius squared | [x] |
| 34 | `c2CircletoAABB` | center x `below`, y `inside`; distance squared `<` radius squared | [x] |
| 35 | `c2CircletoAABB` | center x `below`, y `inside`; distance squared `==` radius squared | [x] |
| 36 | `c2CircletoAABB` | center x `below`, y `inside`; distance squared `>` radius squared | [x] |
| 37 | `c2CircletoAABB` | center x `below`, y `above`; distance squared `<` radius squared | [x] |
| 38 | `c2CircletoAABB` | center x `below`, y `above`; distance squared `==` radius squared | [x] |
| 39 | `c2CircletoAABB` | center x `below`, y `above`; distance squared `>` radius squared | [x] |
| 40 | `c2CircletoAABB` | center x `inside`, y `below`; distance squared `<` radius squared | [x] |
| 41 | `c2CircletoAABB` | center x `inside`, y `below`; distance squared `==` radius squared | [x] |
| 42 | `c2CircletoAABB` | center x `inside`, y `below`; distance squared `>` radius squared | [x] |
| 43 | `c2CircletoAABB` | center x `inside`, y `inside`; zero distance and nonzero radius | [x] |
| 44 | `c2CircletoAABB` | center x `inside`, y `inside`; zero distance and zero radius (strict boundary) | [x] |
| 45 | `c2CircletoAABB` | center x `inside`, y `above`; distance squared `<` radius squared | [x] |
| 46 | `c2CircletoAABB` | center x `inside`, y `above`; distance squared `==` radius squared | [x] |
| 47 | `c2CircletoAABB` | center x `inside`, y `above`; distance squared `>` radius squared | [x] |
| 48 | `c2CircletoAABB` | center x `above`, y `below`; distance squared `<` radius squared | [x] |
| 49 | `c2CircletoAABB` | center x `above`, y `below`; distance squared `==` radius squared | [x] |
| 50 | `c2CircletoAABB` | center x `above`, y `below`; distance squared `>` radius squared | [x] |
| 51 | `c2CircletoAABB` | center x `above`, y `inside`; distance squared `<` radius squared | [x] |
| 52 | `c2CircletoAABB` | center x `above`, y `inside`; distance squared `==` radius squared | [x] |
| 53 | `c2CircletoAABB` | center x `above`, y `inside`; distance squared `>` radius squared | [x] |
| 54 | `c2CircletoAABB` | center x `above`, y `above`; distance squared `<` radius squared | [x] |
| 55 | `c2CircletoAABB` | center x `above`, y `above`; distance squared `==` radius squared | [x] |
| 56 | `c2CircletoAABB` | center x `above`, y `above`; distance squared `>` radius squared or comparison `UNORD` | [x] |
| 57 | `c2CircletoAABB` | one or both AABB axes inverted (`min > max`), exercising clamp's `inverted` branch | [x] |
| 58 | `c2AABBtoAABB` | no separating comparison true; positive-area overlap | [x] |
| 59 | `c2AABBtoAABB` | no separating comparison true; edge or corner equality/touch | [x] |
| 60 | `c2AABBtoAABB` | only `B.max.x < A.min.x` true | [x] |
| 61 | `c2AABBtoAABB` | only `A.max.x < B.min.x` true | [x] |
| 62 | `c2AABBtoAABB` | only `B.max.y < A.min.y` true | [x] |
| 63 | `c2AABBtoAABB` | only `A.max.y < B.min.y` true | [x] |
| 64 | `c2AABBtoAABB` | multiple separating comparisons true, possible with inverted boxes | [x] |
| 65 | `collided` | `typeA=CIRCLE`, `typeB=CIRCLE`; complete dispatcher path | [x] |
| 66 | `collided` | `typeA=CIRCLE`, `typeB=AABB`; complete dispatcher path | [x] |
| 67 | `collided` | `typeA=AABB`, `typeB=CIRCLE`; swapped circle/AABB dispatcher path | [x] |
| 68 | `collided` | `typeA=AABB`, `typeB=AABB`; complete dispatcher path | [x] |

Cargo features: none are declared. The sole effective build configuration is
therefore the same under default and `--no-default-features` invocations.
