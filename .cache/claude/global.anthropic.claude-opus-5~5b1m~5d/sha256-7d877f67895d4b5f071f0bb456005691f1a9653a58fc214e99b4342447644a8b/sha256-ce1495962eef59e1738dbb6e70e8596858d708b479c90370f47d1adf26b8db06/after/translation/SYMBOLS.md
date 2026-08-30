# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C   `.so`: `c_src/build/libharvest-work-aMzaun.so`
* Rust`.so`: `translation/target/release/libreverse_collide_lib.so`

Reproduce with:

```sh
nm -D --defined-only c_src/build/*.so                        | awk '{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libreverse_collide_lib.so | awk '{print $3}' | sort > /tmp/rust_syms.txt
comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt   # missing from Rust  -> MUST be empty
comm -13 /tmp/c_syms.txt /tmp/rust_syms.txt   # extra in Rust      -> informational
```

`c_src/src/lib.c` declares every function with external linkage (no `static`,
no `inline`), so the C `.so` exports all 38 of them, not just the single symbol
declared in `include/lib.h` (`reverse_collide`). The Rust crate must therefore
export all 38.

## Symbol table (38 symbols)

| # | symbol | C signature (from c_src/src/lib.c) | in C .so | in Rust .so |
|---|--------|------------------------------------|----------|-------------|
| 1 | `c2V` | `c2v c2V(float, float)` | yes | yes |
| 2 | `c2Mulvs` | `c2v c2Mulvs(c2v, float)` | yes | yes |
| 3 | `c2Maxv` | `c2v c2Maxv(c2v, c2v)` | yes | yes |
| 4 | `c2Minv` | `c2v c2Minv(c2v, c2v)` | yes | yes |
| 5 | `c2Clampv` | `c2v c2Clampv(c2v, c2v, c2v)` | yes | yes |
| 6 | `c2Sub` | `c2v c2Sub(c2v, c2v)` | yes | yes |
| 7 | `c2Dot` | `float c2Dot(c2v, c2v)` | yes | yes |
| 8 | `c2RotIdentity` | `c2r c2RotIdentity(void)` | yes | yes |
| 9 | `c2xIdentity` | `c2x c2xIdentity(void)` | yes | yes |
| 10 | `c2BBVerts` | `void c2BBVerts(c2v*, c2AABB*)` | yes | yes |
| 11 | `c2MakeProxy` | `void c2MakeProxy(const void*, C2_TYPE, c2Proxy*)` | yes | yes |
| 12 | `c2Len` | `float c2Len(c2v)` | yes | yes |
| 13 | `c2Det2` | `float c2Det2(c2v, c2v)` | yes | yes |
| 14 | `c2GJKSimplexMetric` | `float c2GJKSimplexMetric(c2Simplex*)` | yes | yes |
| 15 | `c2Mulrv` | `c2v c2Mulrv(c2r, c2v)` | yes | yes |
| 16 | `c2Add` | `c2v c2Add(c2v, c2v)` | yes | yes |
| 17 | `c2Mulxv` | `c2v c2Mulxv(c2x, c2v)` | yes | yes |
| 18 | `c22` | `void c22(c2Simplex*)` | yes | yes |
| 19 | `c23` | `void c23(c2Simplex*)` | yes | yes |
| 20 | `c2Neg` | `c2v c2Neg(c2v)` | yes | yes |
| 21 | `c2Skew` | `c2v c2Skew(c2v)` | yes | yes |
| 22 | `c2CCW90` | `c2v c2CCW90(c2v)` | yes | yes |
| 23 | `c2D` | `c2v c2D(c2Simplex*)` | yes | yes |
| 24 | `c2Support` | `int c2Support(const c2v*, int, c2v)` | yes | yes |
| 25 | `c2Witness` | `void c2Witness(c2Simplex*, c2v*, c2v*)` | yes | yes |
| 26 | `c2Div` | `c2v c2Div(c2v, float)` | yes | yes |
| 27 | `c2Norm` | `c2v c2Norm(c2v)` | yes | yes |
| 28 | `c2L` | `c2v c2L(c2Simplex*)` | yes | yes |
| 29 | `c2MulrvT` | `c2v c2MulrvT(c2r, c2v)` | yes | yes |
| 30 | `c2GJK` | `float c2GJK(const void*, C2_TYPE, const c2x*, const void*, C2_TYPE, const c2x*, c2v*, c2v*, int, int*, c2GJKCache*)` | yes | yes |
| 31 | `c2AABBtoAABB` | `int c2AABBtoAABB(c2AABB, c2AABB)` | yes | yes |
| 32 | `c2AABBtoCapsule` | `int c2AABBtoCapsule(c2AABB, c2Capsule)` | yes | yes |
| 33 | `c2CapsuletoCapsule` | `int c2CapsuletoCapsule(c2Capsule, c2Capsule)` | yes | yes |
| 34 | `c2CircletoCircle` | `int c2CircletoCircle(c2Circle, c2Circle)` | yes | yes |
| 35 | `c2CircletoAABB` | `int c2CircletoAABB(c2Circle, c2AABB)` | yes | yes |
| 36 | `c2CircletoCapsule` | `int c2CircletoCapsule(c2Circle, c2Capsule)` | yes | yes |
| 37 | `c2Collided` | `int c2Collided(const void*, C2_TYPE, const void*, C2_TYPE)` | yes | yes |
| 38 | `reverse_collide` | `int reverse_collide(float, float, float)` | yes | yes |

## Result

```
comm -23 c_syms rust_syms  ->  (empty)
comm -13 c_syms rust_syms  ->  (empty)
```

**0 missing, 0 extra.** No C source file was skipped: `c_src/src/lib.c` is the
only translation unit, and all 38 of its external functions are translated and
exported. There are no undefined non-libc symbols in the Rust `.so`
(`nm -D -u` shows only the usual `libc`/`libgcc` runtime imports).

## ABI notes verified while mapping the surface

* `C2_TYPE` is an all-non-negative C enum → GCC picks `unsigned int` as the
  underlying type; on x86-64 SysV it is passed identically to `int`. Rust uses
  `c_uint`. Out-of-range values therefore travel across FFI unchanged (see
  `ERRORS.md`).
* `c2v` (8 B, 2×float) and `c2r` (8 B) return in a single packed `xmm0`.
* `c2x` (16 B, 4×float) returns in `xmm0`/`xmm1`.
* `c2Circle` (12 B) and `c2AABB` (16 B) are passed in SSE registers;
  `c2Capsule` (20 B) exceeds 16 B and is passed in MEMORY.
* `c2Simplex` in C is `c2sv a, b, c, d; float div; int count;` and the code
  takes `&s.a` and indexes it as an array. The Rust translation models this as
  `verts: [c2sv; 4]` followed by `div`, `count`, which is layout-identical
  (`c2sv` = 36 B, align 4 → simplex = 152 B, align 4). Verified by
  `test_layout_sizes`.
