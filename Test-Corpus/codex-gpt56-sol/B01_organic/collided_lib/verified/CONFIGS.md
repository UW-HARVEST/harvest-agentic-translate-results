# Configuration Surface

The CMake file has no build options or conditional sources, and Cargo has no
features. The only build-time configuration is the empty feature set. Runtime
axes below come directly from the comparisons and switches in `src/lib.c`.
Floating-point generators include finite values, signed zero, infinities, and
NaNs; each row is exercised with a fixed-seed randomized corpus.

For clamp-derived rows, a coordinate state is:

- `0`: `a < hi` and `lo <= a` (the input coordinate is selected)
- `1`: `a < hi` and `lo > a` (the low coordinate is selected)
- `2`: `a >= hi` and `lo <= hi` (the high coordinate is selected)
- `3`: `a >= hi` and `lo > hi` (the low coordinate is selected)

| # | entry point(s) | configuration (options set + input shape) | Covered |
|---|----------------|--------------------------------------------|---------|
| 1 | `c2V` | arbitrary `x` and `y` float bit patterns | [x] |
| 2 | `c2Maxv` | `a.x > b.x`, `a.y > b.y` | [x] |
| 3 | `c2Maxv` | `a.x > b.x`, `a.y <= b.y` or unordered | [x] |
| 4 | `c2Maxv` | `a.x <= b.x` or unordered, `a.y > b.y` | [x] |
| 5 | `c2Maxv` | both comparisons false, including equal and unordered operands | [x] |
| 6 | `c2Minv` | `a.x < b.x`, `a.y < b.y` | [x] |
| 7 | `c2Minv` | `a.x < b.x`, `a.y >= b.y` or unordered | [x] |
| 8 | `c2Minv` | `a.x >= b.x` or unordered, `a.y < b.y` | [x] |
| 9 | `c2Minv` | both comparisons false, including equal and unordered operands | [x] |
| 10 | `c2Clampv` | x state 0, y state 0 | [x] |
| 11 | `c2Clampv` | x state 0, y state 1 | [x] |
| 12 | `c2Clampv` | x state 0, y state 2 | [x] |
| 13 | `c2Clampv` | x state 0, y state 3 | [x] |
| 14 | `c2Clampv` | x state 1, y state 0 | [x] |
| 15 | `c2Clampv` | x state 1, y state 1 | [x] |
| 16 | `c2Clampv` | x state 1, y state 2 | [x] |
| 17 | `c2Clampv` | x state 1, y state 3 | [x] |
| 18 | `c2Clampv` | x state 2, y state 0 | [x] |
| 19 | `c2Clampv` | x state 2, y state 1 | [x] |
| 20 | `c2Clampv` | x state 2, y state 2 | [x] |
| 21 | `c2Clampv` | x state 2, y state 3 | [x] |
| 22 | `c2Clampv` | x state 3, y state 0 | [x] |
| 23 | `c2Clampv` | x state 3, y state 1 | [x] |
| 24 | `c2Clampv` | x state 3, y state 2 | [x] |
| 25 | `c2Clampv` | x state 3, y state 3 | [x] |
| 26 | `c2Sub` | arbitrary vector operands | [x] |
| 27 | `c2Dot` | arbitrary vector operands; C multiply-then-add evaluation order | [x] |
| 28 | `c2CircletoCircle` | squared center distance is less than squared radius sum | [x] |
| 29 | `c2CircletoCircle` | squared center distance is not less (equal, greater, or unordered) | [x] |
| 30 | `c2CircletoAABB` | x state 0, y state 0; squared distance is less than squared radius | [x] |
| 31 | `c2CircletoAABB` | x state 0, y state 1; squared distance is less than squared radius | [x] |
| 32 | `c2CircletoAABB` | x state 0, y state 2; squared distance is less than squared radius | [x] |
| 33 | `c2CircletoAABB` | x state 0, y state 3; squared distance is less than squared radius | [x] |
| 34 | `c2CircletoAABB` | x state 1, y state 0; squared distance is less than squared radius | [x] |
| 35 | `c2CircletoAABB` | x state 1, y state 1; squared distance is less than squared radius | [x] |
| 36 | `c2CircletoAABB` | x state 1, y state 2; squared distance is less than squared radius | [x] |
| 37 | `c2CircletoAABB` | x state 1, y state 3; squared distance is less than squared radius | [x] |
| 38 | `c2CircletoAABB` | x state 2, y state 0; squared distance is less than squared radius | [x] |
| 39 | `c2CircletoAABB` | x state 2, y state 1; squared distance is less than squared radius | [x] |
| 40 | `c2CircletoAABB` | x state 2, y state 2; squared distance is less than squared radius | [x] |
| 41 | `c2CircletoAABB` | x state 2, y state 3; squared distance is less than squared radius | [x] |
| 42 | `c2CircletoAABB` | x state 3, y state 0; squared distance is less than squared radius | [x] |
| 43 | `c2CircletoAABB` | x state 3, y state 1; squared distance is less than squared radius | [x] |
| 44 | `c2CircletoAABB` | x state 3, y state 2; squared distance is less than squared radius | [x] |
| 45 | `c2CircletoAABB` | x state 3, y state 3; squared distance is less than squared radius | [x] |
| 46 | `c2CircletoAABB` | x state 0, y state 0; squared distance is not less | [x] |
| 47 | `c2CircletoAABB` | x state 0, y state 1; squared distance is not less | [x] |
| 48 | `c2CircletoAABB` | x state 0, y state 2; squared distance is not less | [x] |
| 49 | `c2CircletoAABB` | x state 0, y state 3; squared distance is not less | [x] |
| 50 | `c2CircletoAABB` | x state 1, y state 0; squared distance is not less | [x] |
| 51 | `c2CircletoAABB` | x state 1, y state 1; squared distance is not less | [x] |
| 52 | `c2CircletoAABB` | x state 1, y state 2; squared distance is not less | [x] |
| 53 | `c2CircletoAABB` | x state 1, y state 3; squared distance is not less | [x] |
| 54 | `c2CircletoAABB` | x state 2, y state 0; squared distance is not less | [x] |
| 55 | `c2CircletoAABB` | x state 2, y state 1; squared distance is not less | [x] |
| 56 | `c2CircletoAABB` | x state 2, y state 2; squared distance is not less | [x] |
| 57 | `c2CircletoAABB` | x state 2, y state 3; squared distance is not less | [x] |
| 58 | `c2CircletoAABB` | x state 3, y state 0; squared distance is not less | [x] |
| 59 | `c2CircletoAABB` | x state 3, y state 1; squared distance is not less | [x] |
| 60 | `c2CircletoAABB` | x state 3, y state 2; squared distance is not less | [x] |
| 61 | `c2CircletoAABB` | x state 3, y state 3; squared distance is not less | [x] |
| 62 | `c2AABBtoAABB` | separation mask `0000` (`d0`, `d1`, `d2`, `d3` all false) | [x] |
| 63 | `c2AABBtoAABB` | separation mask `0001` | [x] |
| 64 | `c2AABBtoAABB` | separation mask `0010` | [x] |
| 65 | `c2AABBtoAABB` | separation mask `0011` | [x] |
| 66 | `c2AABBtoAABB` | separation mask `0100` | [x] |
| 67 | `c2AABBtoAABB` | separation mask `0101` | [x] |
| 68 | `c2AABBtoAABB` | separation mask `0110` | [x] |
| 69 | `c2AABBtoAABB` | separation mask `0111` | [x] |
| 70 | `c2AABBtoAABB` | separation mask `1000` | [x] |
| 71 | `c2AABBtoAABB` | separation mask `1001` | [x] |
| 72 | `c2AABBtoAABB` | separation mask `1010` | [x] |
| 73 | `c2AABBtoAABB` | separation mask `1011` | [x] |
| 74 | `c2AABBtoAABB` | separation mask `1100` | [x] |
| 75 | `c2AABBtoAABB` | separation mask `1101` | [x] |
| 76 | `c2AABBtoAABB` | separation mask `1110` | [x] |
| 77 | `c2AABBtoAABB` | separation mask `1111` | [x] |
| 78 | `collided` | `typeA = CIRCLE`, `typeB = CIRCLE`; full pointer-based dispatch | [x] |
| 79 | `collided` | `typeA = CIRCLE`, `typeB = AABB`; full pointer-based dispatch | [x] |
| 80 | `collided` | `typeA = AABB`, `typeB = CIRCLE`; reversed mixed-shape dispatch | [x] |
| 81 | `collided` | `typeA = AABB`, `typeB = AABB`; full pointer-based dispatch | [x] |
