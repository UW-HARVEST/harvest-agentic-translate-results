# SYMBOLS.md — Phase A symbol surface

C `.so`: `c_src/build/libharvest-work-7PcXTT.so`
Rust `.so`: `translation/target/release/libomni_manifold_lib.so`

Generated with:

```sh
nm -D --defined-only <so> | awk '{print $3}' | sort
```

`nm -D` reports **46** defined dynamic symbols for the C library and **46** for
the Rust library. `comm -23 c_syms r_syms` is **empty** — no missing symbols.

| # | symbol | C | Rust | notes |
|---|--------|---|------|-------|
| 1 | `c22` | ✔ | ✔ | `void c22(c2Simplex*)` |
| 2 | `c23` | ✔ | ✔ | `void c23(c2Simplex*)` |
| 3 | `c2AABBtoAABBManifold` | ✔ | ✔ | struct-by-value ×2 + out ptr |
| 4 | `c2AABBtoCapsuleManifold` | ✔ | ✔ | struct-by-value ×2 + out ptr |
| 5 | `c2Absv` | ✔ | ✔ | |
| 6 | `c2Add` | ✔ | ✔ | |
| 7 | `c2BBVerts` | ✔ | ✔ | |
| 8 | `c2CCW90` | ✔ | ✔ | |
| 9 | `c2CapsuletoCapsuleManifold` | ✔ | ✔ | |
| 10 | `c2CapsuletoPolyManifold` | ✔ | ✔ | takes `const c2Poly*`, `const c2x*` |
| 11 | `c2CircletoAABBManifold` | ✔ | ✔ | |
| 12 | `c2CircletoCapsuleManifold` | ✔ | ✔ | |
| 13 | `c2CircletoCircleManifold` | ✔ | ✔ | |
| 14 | `c2Clampv` | ✔ | ✔ | |
| 15 | `c2Collide` | ✔ | ✔ | void*/enum dispatch |
| 16 | `c2D` | ✔ | ✔ | |
| 17 | `c2Det2` | ✔ | ✔ | |
| 18 | `c2Dist` | ✔ | ✔ | `c2h` by value |
| 19 | `c2Div` | ✔ | ✔ | |
| 20 | `c2Dot` | ✔ | ✔ | |
| 21 | `c2GJK` | ✔ | ✔ | 11 params, returns float |
| 22 | `c2GJKSimplexMetric` | ✔ | ✔ | |
| 23 | `c2Intersect` | ✔ | ✔ | |
| 24 | `c2L` | ✔ | ✔ | |
| 25 | `c2Len` | ✔ | ✔ | |
| 26 | `c2MakeProxy` | ✔ | ✔ | no `C2_TYPE_POLY` case |
| 27 | `c2Maxv` | ✔ | ✔ | |
| 28 | `c2Minv` | ✔ | ✔ | |
| 29 | `c2Mulrv` | ✔ | ✔ | |
| 30 | `c2MulrvT` | ✔ | ✔ | |
| 31 | `c2Mulvs` | ✔ | ✔ | |
| 32 | `c2Mulxv` | ✔ | ✔ | `c2x` by value (16B, SSE/SSE) |
| 33 | `c2MulxvT` | ✔ | ✔ | |
| 34 | `c2Neg` | ✔ | ✔ | |
| 35 | `c2Norm` | ✔ | ✔ | |
| 36 | `c2Norms` | ✔ | ✔ | |
| 37 | `c2PlaneAt` | ✔ | ✔ | returns `c2h` (12B) |
| 38 | `c2RotIdentity` | ✔ | ✔ | |
| 39 | `c2Skew` | ✔ | ✔ | |
| 40 | `c2Sub` | ✔ | ✔ | |
| 41 | `c2Support` | ✔ | ✔ | |
| 42 | `c2V` | ✔ | ✔ | |
| 43 | `c2Witness` | ✔ | ✔ | |
| 44 | `c2xIdentity` | ✔ | ✔ | returns `c2x` |
| 45 | `omni_manifold` | ✔ | ✔ | public header entry point |
| 46 | `ptr_from_parts` | ✔ | ✔ | `malloc`s, no `default:` case |

`static` C functions (correctly NOT exported by either library): `c2Clip`,
`c2SidePlanes`, `c2SidePlanesFromPoly`, `c2KeepDeep`, `c2Incident`.

## Result

- [x] 0 symbols missing from the Rust `.so`.
- [x] 0 undefined non-libc symbols in the Rust `.so` (`nm -D -u` shows only
      libc/`libgcc`/rust-std imports).
