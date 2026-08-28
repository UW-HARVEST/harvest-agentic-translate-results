# Dynamic Symbol Surface

Generated from:

```sh
nm -D --defined-only ../c_src/build/libharvest-work-80YgMl.so
nm -D --defined-only target/release/libagglom_lib.so
```

Only defined dynamic symbols are public library entry points. Addresses are
intentionally omitted because they vary by build.

| # | C symbol | C type | Rust export |
|---|----------|--------|-------------|
| 1 | `c2V` | `T` | [x] |
| 2 | `c2Maxv` | `T` | [x] |
| 3 | `c2Minv` | `T` | [x] |
| 4 | `c2Clampv` | `T` | [x] |
| 5 | `c2Sub` | `T` | [x] |
| 6 | `c2Dot` | `T` | [x] |
| 7 | `c2CircletoCircle` | `T` | [x] |
| 8 | `c2CircletoAABB` | `T` | [x] |
| 9 | `c2AABBtoAABB` | `T` | [x] |
| 10 | `f2` | `T` | [x] |
| 11 | `f3` | `T` | [x] |
| 12 | `f4` | `T` | [x] |
| 13 | `f5` | `T` | [x] |
| 14 | `f7` | `T` | [x] |
| 15 | `f9` | `T` | [x] |
| 16 | `f10` | `T` | [x] |
| 17 | `f11` | `T` | [x] |
| 18 | `f12` | `T` | [x] |
| 19 | `f13` | `T` | [x] |
| 20 | `agglom` | `T` | [x] |

Missing C symbols in Rust: **0**.

The Rust object has no undefined project symbols. Its undefined dynamic symbols
are runtime/libc/libgcc imports supplied by the platform dynamic loader.
