# Dynamic Symbol Surface

Source library: `c_src/build/libtranslated_rust.so`

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

Only globally defined dynamic symbols are part of the library API table.
All 20 C symbols are present with the exact same name in
`target/release/libagglom_lib.so`.

| # | C symbol | ELF type | Rust export | Status |
|---|----------|----------|-------------|--------|
| 1 | `agglom` | `T` | `agglom` | present |
| 2 | `c2AABBtoAABB` | `T` | `c2AABBtoAABB` | present |
| 3 | `c2CircletoAABB` | `T` | `c2CircletoAABB` | present |
| 4 | `c2CircletoCircle` | `T` | `c2CircletoCircle` | present |
| 5 | `c2Clampv` | `T` | `c2Clampv` | present |
| 6 | `c2Dot` | `T` | `c2Dot` | present |
| 7 | `c2Maxv` | `T` | `c2Maxv` | present |
| 8 | `c2Minv` | `T` | `c2Minv` | present |
| 9 | `c2Sub` | `T` | `c2Sub` | present |
| 10 | `c2V` | `T` | `c2V` | present |
| 11 | `f10` | `T` | `f10` | present |
| 12 | `f11` | `T` | `f11` | present |
| 13 | `f12` | `T` | `f12` | present |
| 14 | `f13` | `T` | `f13` | present |
| 15 | `f2` | `T` | `f2` | present |
| 16 | `f3` | `T` | `f3` | present |
| 17 | `f4` | `T` | `f4` | present |
| 18 | `f5` | `T` | `f5` | present |
| 19 | `f7` | `T` | `f7` | present |
| 20 | `f9` | `T` | `f9` | present |

Missing C symbols in Rust: **0**

Unexpected Rust API symbols: **0**

## Completion

- [x] Every globally defined C dynamic symbol has an exact-name Rust export.
- [x] No C API symbol is unresolved in the Rust library.
- [x] Rust's remaining undefined entries are normal platform/runtime imports
  resolved by its declared ELF dependencies.
