# SYMBOLS.md — Phase A symbol surface

C shared library: `c_src/build/libharvest-work-T86gIO.so`
Rust shared library: `translation/target/release/libspec_ray_lib.so`

Command used:

```sh
nm -D --defined-only <so> | awk '{print $3}' | sort
comm -23 c_syms.txt r_syms.txt   # symbols in C but not Rust
```

## Exported symbols (22 total)

| # | symbol | C `.so` | Rust `.so` | C signature (from `src/lib.c`) | ABI class notes |
|---|--------|---------|------------|--------------------------------|-----------------|
| 1 | `c2V` | ✅ | ✅ | `c2v c2V(float, float)` | 8-byte struct ret in `xmm0` |
| 2 | `c2Dot` | ✅ | ✅ | `float c2Dot(c2v, c2v)` | args in `xmm0`,`xmm1` |
| 3 | `c2Len` | ✅ | ✅ | `float c2Len(c2v)` | |
| 4 | `c2Add` | ✅ | ✅ | `c2v c2Add(c2v, c2v)` | |
| 5 | `c2Sub` | ✅ | ✅ | `c2v c2Sub(c2v, c2v)` | |
| 6 | `c2Mulvs` | ✅ | ✅ | `c2v c2Mulvs(c2v, float)` | |
| 7 | `c2Div` | ✅ | ✅ | `c2v c2Div(c2v, float)` | multiplies by `1.0f/b` |
| 8 | `c2Norm` | ✅ | ✅ | `c2v c2Norm(c2v)` | |
| 9 | `c2Minv` | ✅ | ✅ | `c2v c2Minv(c2v, c2v)` | ternary min, not `fminf` |
| 10 | `c2Maxv` | ✅ | ✅ | `c2v c2Maxv(c2v, c2v)` | ternary max, not `fmaxf` |
| 11 | `c2Skew` | ✅ | ✅ | `c2v c2Skew(c2v)` | |
| 12 | `c2Absv` | ✅ | ✅ | `c2v c2Absv(c2v)` | ternary abs, not `fabsf` |
| 13 | `c2CCW90` | ✅ | ✅ | `c2v c2CCW90(c2v)` | |
| 14 | `c2MulmvT` | ✅ | ✅ | `c2v c2MulmvT(c2m, c2v)` | `c2m` = 16 B, `xmm0`+`xmm1` |
| 15 | `c2AABBtoAABB` | ✅ | ✅ | `int c2AABBtoAABB(c2AABB, c2AABB)` | 16-B structs |
| 16 | `c2AABBtoPoint` | ✅ | ✅ | `int c2AABBtoPoint(c2AABB, c2v)` | |
| 17 | `c2CircleToPoint` | ✅ | ✅ | `int c2CircleToPoint(c2Circle, c2v)` | `c2Circle` 12 B → SSE,SSE |
| 18 | `c2RaytoCircle` | ✅ | ✅ | `int c2RaytoCircle(c2Ray, c2Circle, c2Raycast*)` | `c2Ray` 20 B → **MEMORY** (stack) |
| 19 | `c2RaytoAABB` | ✅ | ✅ | `int c2RaytoAABB(c2Ray, c2AABB, c2Raycast*)` | `c2Ray` on stack |
| 20 | `c2RaytoCapsule` | ✅ | ✅ | `int c2RaytoCapsule(c2Ray, c2Capsule, c2Raycast*)` | both 20 B → stack |
| 21 | `c2CastRay` | ✅ | ✅ | `int c2CastRay(c2Ray, const void*, C2_TYPE, c2Raycast*)` | `A` on stack; `rdi`=B, `esi`=type, `rdx`=out |
| 22 | `spec_ray` | ✅ | ✅ | `int spec_ray(c2Raycast*, float×7)` | the `include/lib.h` entry point |

## `static` (non-exported) functions — must NOT appear in either `.so`

| symbol | in C `.so` | in Rust `.so` |
|--------|-----------|---------------|
| `c2SignedDistPointToPlane_OneDimensional` | ❌ (static inline) | ❌ (private `fn`) |
| `c2RayToPlane_OneDimensional` | ❌ (static inline) | ❌ (private `fn`) |

## Result

`comm -23 c_syms.txt r_syms.txt` → **empty**. 0 missing symbols, 0 undefined
non-libc symbols in the Rust `.so`. No module of `c_src/src/lib.c` was skipped:
the C translation unit contains exactly 22 external functions + 2 `static
inline` helpers, and all 24 are present in `translation/src/lib.rs`.

No binary/driver executable is produced by `c_src/CMakeLists.txt` (only
`add_library(... SHARED ...)`), so the "compare binary stdout" gate is N/A.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** → the only build
configuration is the default one. Verified with
`cargo check --no-default-features` (identical result).

---

## Phase D final result (re-verified)

```
$ nm -D --defined-only c_src/build/libharvest-work-T86gIO.so | awk '{print $3}' | sort -u > c_syms
$ nm -D --defined-only translation/target/release/libspec_ray_lib.so | awk '{print $3}' | sort -u > r_syms
$ comm -23 c_syms r_syms          # C symbols missing from Rust
(empty)
$ ldd -r translation/target/release/libspec_ray_lib.so | grep -i 'undefined symbol'
(empty)
```

22 / 22 symbols exported with identical names. No stubs, no `unimplemented!()`:
every symbol is backed by a real translation of the corresponding C function.
Confirmed for `default`, `--no-default-features` and `--all-features`, in both
the `debug` and `release` profiles, by `translation/verify_all.sh`.
