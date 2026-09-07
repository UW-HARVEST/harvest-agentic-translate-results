# SYMBOLS.md — exported-symbol parity

Derived mechanically:

```
nm -D --defined-only c_src/build/libharvest-work-wd9wzk.so | awk '{print $3}' | sort
nm -D --defined-only translation/target/release/libreverse_collide_lib.so | awk '{print $3}' | sort
comm -23 c_syms.txt rust_syms.txt   # missing in Rust  -> EMPTY
comm -13 c_syms.txt rust_syms.txt   # extra in Rust    -> EMPTY
```

C `.so` exports 38 `T` symbols. Rust `.so` exports the same 38. **Diff is empty.**

| # | symbol | in C .so | in Rust .so | notes |
|---|--------|----------|-------------|-------|
| 1 | `c2V` | T | T | |
| 2 | `c2Mulvs` | T | T | |
| 3 | `c2Maxv` | T | T | |
| 4 | `c2Minv` | T | T | |
| 5 | `c2Clampv` | T | T | |
| 6 | `c2Sub` | T | T | |
| 7 | `c2Dot` | T | T | |
| 8 | `c2RotIdentity` | T | T | |
| 9 | `c2xIdentity` | T | T | |
| 10 | `c2BBVerts` | T | T | out-param `c2v*` (4 writes) |
| 11 | `c2MakeProxy` | T | T | `switch` on `C2_TYPE`, no default arm |
| 12 | `c2Len` | T | T | calls `sqrtf` |
| 13 | `c2Det2` | T | T | |
| 14 | `c2GJKSimplexMetric` | T | T | takes `c2Simplex*` |
| 15 | `c2Mulrv` | T | T | |
| 16 | `c2Add` | T | T | |
| 17 | `c2Mulxv` | T | T | |
| 18 | `c22` | T | T | 2-simplex solver |
| 19 | `c23` | T | T | 3-simplex solver |
| 20 | `c2Neg` | T | T | |
| 21 | `c2Skew` | T | T | |
| 22 | `c2CCW90` | T | T | |
| 23 | `c2D` | T | T | search direction |
| 24 | `c2Support` | T | T | |
| 25 | `c2Witness` | T | T | two out-params |
| 26 | `c2Div` | T | T | |
| 27 | `c2Norm` | T | T | |
| 28 | `c2L` | T | T | |
| 29 | `c2MulrvT` | T | T | |
| 30 | `c2GJK` | T | T | 11 parameters |
| 31 | `c2AABBtoAABB` | T | T | |
| 32 | `c2AABBtoCapsule` | T | T | |
| 33 | `c2CapsuletoCapsule` | T | T | |
| 34 | `c2CircletoCircle` | T | T | |
| 35 | `c2CircletoAABB` | T | T | |
| 36 | `c2CircletoCapsule` | T | T | |
| 37 | `c2Collided` | T | T | dispatcher |
| 38 | `reverse_collide` | T | T | the only symbol in `include/lib.h` |

## Undefined (imported) symbols

C `.so`: `sqrtf` + the usual weak glibc/ITM stubs.
Rust `.so`: only libc (`memcpy`, `malloc`, …) and `_Unwind_*` / `__cxa_*`
runtime symbols. **0 missing/undefined non-libc symbols.**

No module of the C source was skipped: `c_src/src/lib.c` is the single
translation unit and every one of its 38 external-linkage functions has a real
Rust body (no stubs, no `unimplemented!()`).

## ABI notes verified by the tests

* `C2_TYPE` is an all-non-negative C enum → GCC underlying type `unsigned int`,
  passed as a 32-bit register value; the Rust side uses `c_uint`.
* `c2v` (8 bytes, 2×f32) returns/passes in one SSE register.
* `c2Capsule` (20 bytes) exceeds the 16-byte SSE class limit → passed in memory.
* `c2Simplex` in Rust uses `verts: [c2sv; 4]` instead of the four named members
  `a, b, c, d`; `c2sv` is 36 bytes / align 4, so the layout is identical and the
  C idiom `c2sv *verts = &s.a;` maps to indexing that array.
