# Dynamic Symbol Surface

Source command:

```sh
nm -D --defined-only ../c_src/build/libharvest-work-bfWKpd.so
```

The C shared library exports ten functions. The Rust status was determined by
exact-name comparison with `target/release/libcollided_lib.so`.

| # | C symbol | C source | Rust export |
|---|----------|----------|-------------|
| 1 | `c2V` | `src/lib.c:18` | present |
| 2 | `c2Maxv` | `src/lib.c:25` | present |
| 3 | `c2Minv` | `src/lib.c:30` | present |
| 4 | `c2Clampv` | `src/lib.c:35` | present |
| 5 | `c2Sub` | `src/lib.c:39` | present |
| 6 | `c2Dot` | `src/lib.c:45` | present |
| 7 | `c2CircletoCircle` | `src/lib.c:49` | present |
| 8 | `c2CircletoAABB` | `src/lib.c:57` | present |
| 9 | `c2AABBtoAABB` | `src/lib.c:65` | present |
| 10 | `collided` | `src/lib.c:73` | present |

Missing C symbols in Rust: **0**.

Undefined non-libc C symbols in Rust: **0**.
