# Configuration Surface

Derived from all exported C entry points and every data-dependent `?:`, `if`,
and `switch` branch in `c_src/src/lib.c`. There are no compile-time or runtime
feature flags. `low`, `inside`, and `high` below are per-axis positions relative
to ordered AABB/clamp bounds. `overlap`, `tangent`, and `separate` mean `<`,
`==`, and `>` respectively for the exact squared-distance comparison used by C.
False comparison arms include equality and unordered (NaN) comparisons.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `c2V` | Arbitrary `x,y` bit patterns: finite, signed zero, infinity, NaN | [x] |
| 2 | `c2Mulvs` | Arbitrary vector and scalar; no data-dependent branch | [x] |
| 3 | `c2Maxv` | `a.x > b.x`, `a.y > b.y` | [x] |
| 4 | `c2Maxv` | `a.x > b.x`, `a.y <= b.y` or unordered | [x] |
| 5 | `c2Maxv` | `a.x <= b.x` or unordered, `a.y > b.y` | [x] |
| 6 | `c2Maxv` | Both comparisons false (less, equal, or unordered) | [x] |
| 7 | `c2Minv` | `a.x < b.x`, `a.y < b.y` | [x] |
| 8 | `c2Minv` | `a.x < b.x`, `a.y >= b.y` or unordered | [x] |
| 9 | `c2Minv` | `a.x >= b.x` or unordered, `a.y < b.y` | [x] |
| 10 | `c2Minv` | Both comparisons false (greater, equal, or unordered) | [x] |
| 11 | `c2Clampv` | x low, y low, ordered bounds | [x] |
| 12 | `c2Clampv` | x low, y inside, ordered bounds | [x] |
| 13 | `c2Clampv` | x low, y high, ordered bounds | [x] |
| 14 | `c2Clampv` | x inside, y low, ordered bounds | [x] |
| 15 | `c2Clampv` | x inside, y inside, ordered bounds | [x] |
| 16 | `c2Clampv` | x inside, y high, ordered bounds | [x] |
| 17 | `c2Clampv` | x high, y low, ordered bounds | [x] |
| 18 | `c2Clampv` | x high, y inside, ordered bounds | [x] |
| 19 | `c2Clampv` | x high, y high, ordered bounds | [x] |
| 20 | `c2Clampv` | Reversed/equal/non-finite bounds, exercising false comparison arms | [x] |
| 21 | `c2Sub` | Arbitrary vectors; no data-dependent branch | [x] |
| 22 | `c2Dot` | Arbitrary vectors, including cancellation and non-finite products | [x] |
| 23 | `c2CircletoCircle` | squared distance `<` squared radius sum (overlap) | [x] |
| 24 | `c2CircletoCircle` | squared distance `==` squared radius sum (tangent, false) | [x] |
| 25 | `c2CircletoCircle` | squared distance `>` squared radius sum (separate) | [x] |
| 26 | `c2CircletoAABB` | x low, y low; overlap | [x] |
| 27 | `c2CircletoAABB` | x low, y low; tangent | [x] |
| 28 | `c2CircletoAABB` | x low, y low; separate | [x] |
| 29 | `c2CircletoAABB` | x low, y inside; overlap | [x] |
| 30 | `c2CircletoAABB` | x low, y inside; tangent | [x] |
| 31 | `c2CircletoAABB` | x low, y inside; separate | [x] |
| 32 | `c2CircletoAABB` | x low, y high; overlap | [x] |
| 33 | `c2CircletoAABB` | x low, y high; tangent | [x] |
| 34 | `c2CircletoAABB` | x low, y high; separate | [x] |
| 35 | `c2CircletoAABB` | x inside, y low; overlap | [x] |
| 36 | `c2CircletoAABB` | x inside, y low; tangent | [x] |
| 37 | `c2CircletoAABB` | x inside, y low; separate | [x] |
| 38 | `c2CircletoAABB` | x inside, y inside; overlap | [x] |
| 39 | `c2CircletoAABB` | x inside, y inside; tangent (`r = 0`) | [x] |
| 40 | `c2CircletoAABB` | x inside, y inside; false result from NaN `r` (finite strict separation is impossible at zero distance) | [x] |
| 41 | `c2CircletoAABB` | x inside, y high; overlap | [x] |
| 42 | `c2CircletoAABB` | x inside, y high; tangent | [x] |
| 43 | `c2CircletoAABB` | x inside, y high; separate | [x] |
| 44 | `c2CircletoAABB` | x high, y low; overlap | [x] |
| 45 | `c2CircletoAABB` | x high, y low; tangent | [x] |
| 46 | `c2CircletoAABB` | x high, y low; separate | [x] |
| 47 | `c2CircletoAABB` | x high, y inside; overlap | [x] |
| 48 | `c2CircletoAABB` | x high, y inside; tangent | [x] |
| 49 | `c2CircletoAABB` | x high, y inside; separate | [x] |
| 50 | `c2CircletoAABB` | x high, y high; overlap | [x] |
| 51 | `c2CircletoAABB` | x high, y high; tangent | [x] |
| 52 | `c2CircletoAABB` | x high, y high; separate | [x] |
| 53 | `c2CircletoCapsule` | `da < 0` (nearest endpoint A); overlap | [x] |
| 54 | `c2CircletoCapsule` | `da < 0` (nearest endpoint A); tangent | [x] |
| 55 | `c2CircletoCapsule` | `da < 0` (nearest endpoint A); separate | [x] |
| 56 | `c2CircletoCapsule` | `da >= 0 && db < 0` (segment interior); overlap | [x] |
| 57 | `c2CircletoCapsule` | `da >= 0 && db < 0` (segment interior); tangent | [x] |
| 58 | `c2CircletoCapsule` | `da >= 0 && db < 0` (segment interior); separate | [x] |
| 59 | `c2CircletoCapsule` | `da >= 0 && db >= 0` (nearest endpoint B); overlap | [x] |
| 60 | `c2CircletoCapsule` | `da >= 0 && db >= 0` (nearest endpoint B); tangent | [x] |
| 61 | `c2CircletoCapsule` | `da >= 0 && db >= 0` (nearest endpoint B); separate | [x] |
| 62 | `c2CircletoCapsule` | Degenerate segment (`a == b`); overlap | [x] |
| 63 | `c2CircletoCapsule` | Degenerate segment (`a == b`); tangent | [x] |
| 64 | `c2CircletoCapsule` | Degenerate segment (`a == b`); separate | [x] |
| 65 | `c2Collided` | `typeB = C2_TYPE_CIRCLE`, valid circle pointers, all contact outcomes | [x] |
| 66 | `c2Collided` | `typeB = C2_TYPE_AABB`, valid circle/AABB pointers, all clamp/contact shapes | [x] |
| 67 | `c2Collided` | `typeB = C2_TYPE_CAPSULE`, valid circle/capsule pointers, all region/contact shapes | [x] |
| 68 | `circle_collide` | Fixed circle + AABB + capsule pipeline; randomized finite input circles and all observed result masks | [x] |
| 69 | `c2CircletoCircle` | Signed zero, infinity, and NaN fields (unordered final comparison) | [x] |
| 70 | `c2CircletoAABB` | Signed zero, infinity, NaN, and reversed/equal AABB bounds | [x] |
| 71 | `c2CircletoCapsule` | Signed zero, infinity, NaN, and degenerate/non-finite segment fields | [x] |
| 72 | `c2Collided`, `circle_collide` | Non-finite public inputs propagated through dispatch/full pipeline | [x] |
