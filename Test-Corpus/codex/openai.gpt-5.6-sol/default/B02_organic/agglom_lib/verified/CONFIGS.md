# Configuration Surface

This table is derived from every defined dynamic entry point in `SYMBOLS.md`
and every data-dependent `if`, ternary, or `switch` in `c_src/src/lib.c`.
There are no compile-time features or `#if` branches in either implementation.
Rows split the branch cross-product only where C behavior differs.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `c2V` | arbitrary finite and non-finite `x`, `y` | [x] |
| C2 | `c2Maxv` | `a.x > b.x`, `a.y > b.y` | [x] |
| C3 | `c2Maxv` | `a.x > b.x`, `a.y <= b.y` | [x] |
| C4 | `c2Maxv` | `a.x <= b.x`, `a.y > b.y` | [x] |
| C5 | `c2Maxv` | `a.x <= b.x`, `a.y <= b.y` (including equality/NaN comparison fallthrough) | [x] |
| C6 | `c2Minv` | `a.x < b.x`, `a.y < b.y` | [x] |
| C7 | `c2Minv` | `a.x < b.x`, `a.y >= b.y` | [x] |
| C8 | `c2Minv` | `a.x >= b.x`, `a.y < b.y` | [x] |
| C9 | `c2Minv` | `a.x >= b.x`, `a.y >= b.y` (including equality/NaN comparison fallthrough) | [x] |
| C10 | `c2Clampv` | each component below `[lo, hi]` | [x] |
| C11 | `c2Clampv` | x below, y inside | [x] |
| C12 | `c2Clampv` | x below, y above | [x] |
| C13 | `c2Clampv` | x inside, y below | [x] |
| C14 | `c2Clampv` | both components inside, including bounds | [x] |
| C15 | `c2Clampv` | x inside, y above | [x] |
| C16 | `c2Clampv` | x above, y below | [x] |
| C17 | `c2Clampv` | x above, y inside | [x] |
| C18 | `c2Clampv` | each component above `[lo, hi]` | [x] |
| C19 | `c2Sub` | arbitrary vectors | [x] |
| C20 | `c2Dot` | arbitrary vectors, including cancellation and signed zero | [x] |
| C21 | `c2CircletoCircle` | center distance squared `< (A.r+B.r)^2` | [x] |
| C22 | `c2CircletoCircle` | center distance squared `== (A.r+B.r)^2` (tangent) | [x] |
| C23 | `c2CircletoCircle` | center distance squared `> (A.r+B.r)^2` | [x] |
| C24 | `c2CircletoAABB` | center inside box and `d2 < r^2` | [x] |
| C25 | `c2CircletoAABB` | center outside one edge and `d2 < r^2` | [x] |
| C26 | `c2CircletoAABB` | center outside a corner and `d2 < r^2` | [x] |
| C27 | `c2CircletoAABB` | center inside/on box and `d2 >= r^2` (zero radius boundary) | [x] |
| C28 | `c2CircletoAABB` | center outside one edge and `d2 >= r^2` | [x] |
| C29 | `c2CircletoAABB` | center outside a corner and `d2 >= r^2` | [x] |
| C30 | `c2AABBtoAABB` | overlap or touch on all axes | [x] |
| C31 | `c2AABBtoAABB` | `B.max.x < A.min.x` only | [x] |
| C32 | `c2AABBtoAABB` | `A.max.x < B.min.x` only | [x] |
| C33 | `c2AABBtoAABB` | `B.max.y < A.min.y` only | [x] |
| C34 | `c2AABBtoAABB` | `A.max.y < B.min.y` only | [x] |
| C35 | `c2AABBtoAABB` | separated on multiple axes | [x] |
| C36 | `f2` | circle/circle tags with matching object layouts | [x] |
| C37 | `f2` | circle/AABB tags with matching object layouts | [x] |
| C38 | `f2` | AABB/circle tags with matching object layouts | [x] |
| C39 | `f2` | AABB/AABB tags with matching object layouts | [x] |
| C40 | `f3` | `v1 >= 0`, `v2 > 0` | [x] |
| C41 | `f3` | `v1 >= 0`, `v2 < 0`, `v2 != INT_MIN` | [x] |
| C42 | `f3` | `v1 >= 0`, `v2 == INT_MIN` | [x] |
| C43 | `f3` | `v1 < 0`, `v1 != INT_MIN`, `v2 > 0`, nonnegative remainder | [x] |
| C44 | `f3` | `v1 < 0`, `v1 != INT_MIN`, `v2 > 0`, negative remainder correction | [x] |
| C45 | `f3` | `v1 < 0`, `v1 != INT_MIN`, `v2 < 0`, `v2 != INT_MIN`, nonnegative remainder | [x] |
| C46 | `f3` | `v1 < 0`, `v1 != INT_MIN`, `v2 < 0`, `v2 != INT_MIN`, negative remainder correction | [x] |
| C47 | `f3` | `v1 < 0`, `v1 != INT_MIN`, `v2 == INT_MIN` | [x] |
| C48 | `f3` | `v1 == INT_MIN`, `v2 > 0` | [x] |
| C49 | `f3` | `v1 == INT_MIN`, `v2 < 0`, `v2 != INT_MIN` | [x] |
| C50 | `f3` | `v1 == INT_MIN`, `v2 == INT_MIN` | [x] |
| C51 | `f4` | arbitrary two-word PRNG state, including zero and max words | [x] |
| C52 | `f5` | low 16 bits arbitrary, high 16 bits zero | [x] |
| C53 | `f5` | high 16 input bits set (discarded by first mask stage) | [x] |
| C54 | `f7` | `channels != 2`, `bitdepth != 32`, ordinary products | [x] |
| C55 | `f7` | `channels != 2`, `bitdepth == 32`, ordinary products | [x] |
| C56 | `f7` | `channels == 2`, `bitdepth != 32`, ordinary products | [x] |
| C57 | `f7` | `channels == 2`, `bitdepth == 32`, ordinary products | [x] |
| C58 | `f7` | zero blocksize/channels/bitdepth boundaries | [x] |
| C59 | `f7` | values whose unsigned intermediate multiplication/addition wraps | [x] |
| C60 | `f9` | nondegenerate triangle, arbitrary point | [x] |
| C61 | `f9` | degenerate/collinear triangle producing zero denominator and non-finite coordinates | [x] |
| C62 | `f10` | positive and negative zero half encodings | [x] |
| C63 | `f10` | positive and negative subnormal half encodings | [x] |
| C64 | `f10` | positive and negative normal half encodings, exponent 1..30 | [x] |
| C65 | `f10` | positive and negative infinity half encodings | [x] |
| C66 | `f10` | positive and negative NaN half encodings with payloads | [x] |
| C67 | `f11` | `s == 0` early return, arbitrary `h`/`l` | [x] |
| C68 | `f11` | `s != 0`, `0 <= h < 60` | [x] |
| C69 | `f11` | `s != 0`, `60 <= h < 120` | [x] |
| C70 | `f11` | `s != 0`, `h < 0` (the literal third C condition) | [x] |
| C71 | `f11` | `s != 0`, `180 <= h < 240` | [x] |
| C72 | `f11` | `s != 0`, `240 <= h < 300` | [x] |
| C73 | `f11` | `s != 0`, `300 <= h < 360` | [x] |
| C74 | `f11` | `s != 0`, else branch (`120 <= h < 180`, `h >= 360`, or NaN) | [x] |
| C75 | `f12` | `s == 0` early return, arbitrary `h`/`v` | [x] |
| C76 | `f12` | `s != 0`, `floor(h/60) == 0` | [x] |
| C77 | `f12` | `s != 0`, `floor(h/60) == 1` | [x] |
| C78 | `f12` | `s != 0`, `floor(h/60) == 2` | [x] |
| C79 | `f12` | `s != 0`, `floor(h/60) == 3` | [x] |
| C80 | `f12` | `s != 0`, `floor(h/60) == 4` | [x] |
| C81 | `f12` | `s != 0`, default integer sector (negative or >= 5) | [x] |
| C82 | `f13` | `delta == 0`, `max != 0` early return | [x] |
| C83 | `f13` | `max == 0` early return | [x] |
| C84 | `f13` | nondegenerate, `r == max`, computed `h >= 0` | [x] |
| C85 | `f13` | nondegenerate, `r == max`, computed `h < 0` then add 360 | [x] |
| C86 | `f13` | nondegenerate, `g == max` | [x] |
| C87 | `f13` | nondegenerate, `b == max` | [x] |
| C88 | `agglom` | full composed operation with ordinary finite inputs | [x] |
| C89 | `agglom` | full composed operation with NaN-producing `f9` and NaN `f10`; `isnan` filters each result | [x] |
| C90 | `agglom` | integer/unsigned boundary values and hue branch boundaries | [x] |
