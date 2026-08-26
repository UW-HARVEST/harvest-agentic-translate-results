# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

The CMake configuration has no options or conditional source files. Cargo has
no `[features]` table, so the only valid feature combination is the empty set
(`--no-default-features`).

| # | C symbol | Rust symbol | Status |
|---|----------|-------------|--------|
| 1 | `c2AABBtoAABB` | `c2AABBtoAABB` | present |
| 2 | `c2CircletoAABB` | `c2CircletoAABB` | present |
| 3 | `c2CircletoCircle` | `c2CircletoCircle` | present |
| 4 | `c2Clampv` | `c2Clampv` | present |
| 5 | `c2Dot` | `c2Dot` | present |
| 6 | `c2Maxv` | `c2Maxv` | present |
| 7 | `c2Minv` | `c2Minv` | present |
| 8 | `c2Sub` | `c2Sub` | present |
| 9 | `c2V` | `c2V` | present |
| 10 | `collided` | `collided` | present |

Missing from Rust: **0**

