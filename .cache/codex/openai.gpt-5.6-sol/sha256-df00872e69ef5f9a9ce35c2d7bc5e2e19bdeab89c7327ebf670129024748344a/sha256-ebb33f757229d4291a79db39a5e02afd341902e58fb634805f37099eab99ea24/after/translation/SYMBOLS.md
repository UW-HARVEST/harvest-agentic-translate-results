# Dynamic symbol surface

Derived with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-cKI30P.so
nm -D --defined-only target/release/libagglom_lib.so
```

Only globally defined dynamic symbols are listed. The C and Rust symbol sets
are identical.

| C symbol | Rust export |
|----------|-------------|
| `agglom` | present |
| `c2AABBtoAABB` | present |
| `c2CircletoAABB` | present |
| `c2CircletoCircle` | present |
| `c2Clampv` | present |
| `c2Dot` | present |
| `c2Maxv` | present |
| `c2Minv` | present |
| `c2Sub` | present |
| `c2V` | present |
| `f10` | present |
| `f11` | present |
| `f12` | present |
| `f13` | present |
| `f2` | present |
| `f3` | present |
| `f4` | present |
| `f5` | present |
| `f7` | present |
| `f9` | present |

- [x] 20 C symbols enumerated.
- [x] 0 C symbols missing from Rust.
- [x] 0 undefined non-libc implementation symbols in Rust.
