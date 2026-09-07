# SYMBOLS.md — exported-symbol parity

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-iYDMnU.so   | awk '{print $3}' | sort
nm -D --defined-only translation/target/release/libomni_manifold_lib.so | awk '{print $3}' | sort
```

C `.so` exports **46** symbols. Rust `.so` exports **46**. `diff` of the two
sorted lists is **empty**.

| # | symbol | in C .so | in Rust .so | notes |
|---|--------|----------|-------------|-------|
| 1 | `c22` | yes | yes | simplex 2-vertex solver |
| 2 | `c23` | yes | yes | simplex 3-vertex solver |
| 3 | `c2AABBtoAABBManifold` | yes | yes | |
| 4 | `c2AABBtoCapsuleManifold` | yes | yes | |
| 5 | `c2Absv` | yes | yes | |
| 6 | `c2Add` | yes | yes | |
| 7 | `c2BBVerts` | yes | yes | writes 4 `c2v` |
| 8 | `c2CCW90` | yes | yes | |
| 9 | `c2CapsuletoCapsuleManifold` | yes | yes | |
| 10 | `c2CapsuletoPolyManifold` | yes | yes | takes `const c2Poly*`, `const c2x*` |
| 11 | `c2CircletoAABBManifold` | yes | yes | |
| 12 | `c2CircletoCapsuleManifold` | yes | yes | |
| 13 | `c2CircletoCircleManifold` | yes | yes | |
| 14 | `c2Clampv` | yes | yes | |
| 15 | `c2Collide` | yes | yes | type-dispatch entry point |
| 16 | `c2D` | yes | yes | search-direction from simplex |
| 17 | `c2Det2` | yes | yes | |
| 18 | `c2Dist` | yes | yes | |
| 19 | `c2Div` | yes | yes | |
| 20 | `c2Dot` | yes | yes | |
| 21 | `c2GJK` | yes | yes | 11-arg lowest-level distance query |
| 22 | `c2GJKSimplexMetric` | yes | yes | |
| 23 | `c2Intersect` | yes | yes | |
| 24 | `c2L` | yes | yes | |
| 25 | `c2Len` | yes | yes | |
| 26 | `c2MakeProxy` | yes | yes | no POLY case in C — reproduced |
| 27 | `c2Maxv` | yes | yes | |
| 28 | `c2Minv` | yes | yes | |
| 29 | `c2Mulrv` | yes | yes | |
| 30 | `c2MulrvT` | yes | yes | |
| 31 | `c2Mulvs` | yes | yes | |
| 32 | `c2Mulxv` | yes | yes | |
| 33 | `c2MulxvT` | yes | yes | |
| 34 | `c2Neg` | yes | yes | |
| 35 | `c2Norm` | yes | yes | divides by `c2Len` — no zero guard in C |
| 36 | `c2Norms` | yes | yes | |
| 37 | `c2PlaneAt` | yes | yes | unchecked index |
| 38 | `c2RotIdentity` | yes | yes | |
| 39 | `c2Skew` | yes | yes | |
| 40 | `c2Sub` | yes | yes | |
| 41 | `c2Support` | yes | yes | unchecked `verts[0]` read |
| 42 | `c2V` | yes | yes | |
| 43 | `c2Witness` | yes | yes | |
| 44 | `c2xIdentity` | yes | yes | |
| 45 | `omni_manifold` | yes | yes | the header's only declared entry point |
| 46 | `ptr_from_parts` | yes | yes | falls off end for POLY in C |

Static (file-local, not exported by either library, therefore not required to be
exported by Rust): `c2Clip`, `c2SidePlanes`, `c2SidePlanesFromPoly`,
`c2KeepDeep`, `c2Incident`. These are exercised indirectly through
`c2CapsuletoPolyManifold` / `c2AABBtoCapsuleManifold`.

## Undefined (imported) symbols

C imports only `malloc`, `sqrtf` plus CRT glue. Rust imports `malloc` plus libc
/ `_Unwind_*` / std-runtime glue. **0 missing or undefined non-libc symbols in
the Rust `.so`.**

## Status

- [x] `nm -D` symbol diff C → Rust is empty (46/46).
- [x] `nm -D --undefined-only` on the Rust `.so` shows only libc/unwind imports.
