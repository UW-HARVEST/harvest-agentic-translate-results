# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-ZwQjiV.so     | awk '$2=="T"||$2=="W"{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libomni_collide_lib.so | awk '$2=="T"||$2=="W"{print $3}' | sort > /tmp/rust_syms.txt
comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt   # missing from Rust
comm -13 /tmp/c_syms.txt /tmp/rust_syms.txt   # extra in Rust
```

C `.so` defined symbols: **39**. Rust `.so` defined symbols: **39**.
Missing from Rust: **0**. Extra in Rust: **0**.

`c_src/` has exactly one translation unit (`src/lib.c`, 645 lines); no C module was
skipped, so no missing-source translation work was required.

| # | symbol | in C `.so` | in Rust `.so` | signature (from `c_src/src/lib.c`) |
|---|--------|-----------|--------------|-------------------------------------|
| 1 | `c22` | T | T | `void c22(c2Simplex*)` |
| 2 | `c23` | T | T | `void c23(c2Simplex*)` |
| 3 | `c2AABBtoAABB` | T | T | `int c2AABBtoAABB(c2AABB, c2AABB)` |
| 4 | `c2AABBtoCapsule` | T | T | `int c2AABBtoCapsule(c2AABB, c2Capsule)` |
| 5 | `c2Add` | T | T | `c2v c2Add(c2v, c2v)` |
| 6 | `c2BBVerts` | T | T | `void c2BBVerts(c2v*, c2AABB*)` |
| 7 | `c2CCW90` | T | T | `c2v c2CCW90(c2v)` |
| 8 | `c2CapsuletoCapsule` | T | T | `int c2CapsuletoCapsule(c2Capsule, c2Capsule)` |
| 9 | `c2CircletoAABB` | T | T | `int c2CircletoAABB(c2Circle, c2AABB)` |
| 10 | `c2CircletoCapsule` | T | T | `int c2CircletoCapsule(c2Circle, c2Capsule)` |
| 11 | `c2CircletoCircle` | T | T | `int c2CircletoCircle(c2Circle, c2Circle)` |
| 12 | `c2Clampv` | T | T | `c2v c2Clampv(c2v, c2v, c2v)` |
| 13 | `c2Collided` | T | T | `int c2Collided(const void*, C2_TYPE, const void*, C2_TYPE)` |
| 14 | `c2D` | T | T | `c2v c2D(c2Simplex*)` |
| 15 | `c2Det2` | T | T | `float c2Det2(c2v, c2v)` |
| 16 | `c2Div` | T | T | `c2v c2Div(c2v, float)` |
| 17 | `c2Dot` | T | T | `float c2Dot(c2v, c2v)` |
| 18 | `c2GJK` | T | T | `float c2GJK(const void*, C2_TYPE, const c2x*, const void*, C2_TYPE, const c2x*, c2v*, c2v*, int, int*, c2GJKCache*)` |
| 19 | `c2GJKSimplexMetric` | T | T | `float c2GJKSimplexMetric(c2Simplex*)` |
| 20 | `c2L` | T | T | `c2v c2L(c2Simplex*)` |
| 21 | `c2Len` | T | T | `float c2Len(c2v)` |
| 22 | `c2MakeProxy` | T | T | `void c2MakeProxy(const void*, C2_TYPE, c2Proxy*)` |
| 23 | `c2Maxv` | T | T | `c2v c2Maxv(c2v, c2v)` |
| 24 | `c2Minv` | T | T | `c2v c2Minv(c2v, c2v)` |
| 25 | `c2Mulrv` | T | T | `c2v c2Mulrv(c2r, c2v)` |
| 26 | `c2MulrvT` | T | T | `c2v c2MulrvT(c2r, c2v)` |
| 27 | `c2Mulvs` | T | T | `c2v c2Mulvs(c2v, float)` |
| 28 | `c2Mulxv` | T | T | `c2v c2Mulxv(c2x, c2v)` |
| 29 | `c2Neg` | T | T | `c2v c2Neg(c2v)` |
| 30 | `c2Norm` | T | T | `c2v c2Norm(c2v)` |
| 31 | `c2RotIdentity` | T | T | `c2r c2RotIdentity(void)` |
| 32 | `c2Skew` | T | T | `c2v c2Skew(c2v)` |
| 33 | `c2Sub` | T | T | `c2v c2Sub(c2v, c2v)` |
| 34 | `c2Support` | T | T | `int c2Support(const c2v*, int, c2v)` |
| 35 | `c2V` | T | T | `c2v c2V(float, float)` |
| 36 | `c2Witness` | T | T | `void c2Witness(c2Simplex*, c2v*, c2v*)` |
| 37 | `c2xIdentity` | T | T | `c2x c2xIdentity(void)` |
| 38 | `omni_collide` | T | T | `int omni_collide(C2_TYPE, float x5, C2_TYPE, float x5)` |
| 39 | `ptr_from_parts` | T | T | `void* ptr_from_parts(C2_TYPE, float, float, float, float, float)` |

## Undefined symbols in the Rust `.so`

All undefined symbols are libc / libgcc-unwind / Rust-runtime imports
(`malloc`, `free`, `memcpy`, `_Unwind_*`, `__cxa_*`, `pthread_key_*`, …).
Zero undefined non-libc symbols. Note the C `.so` links `-lm` for `sqrtf`;
Rust lowers `f32::sqrt` to the `sqrtss` instruction so no `libm` import
appears — this is an implementation detail, not an ABI difference.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default one. Phase D's "every feature combination"
requirement is satisfied by the single default combination; verified with
`cargo check --no-default-features` as well.

## ABI notes checked while building this table

* `C2_TYPE` is a plain C enum → `int` in the SysV ABI. Rust models it as
  `c_int` with `C2_TYPE_CAPSULE = 0`, `C2_TYPE_CIRCLE = 1`, `C2_TYPE_AABB = 2`,
  matching the declaration order in `c_src/include/lib.h`. **Note the order:
  CAPSULE is 0, not CIRCLE.**
* `c2Simplex` is `{ c2sv a, b, c, d; float div; int count; }` in C and
  `{ [c2sv; 4] verts; f32 div; c_int count; }` in Rust. `c2sv` is 36 bytes
  (align 4), so both layouts are `verts @ 0..144`, `div @ 144`, `count @ 148`,
  size 152. The C code walks the four vertices with `c2sv *verts = &s.a;`,
  which the array reproduces exactly.
* `c2v` (8 bytes, two floats) is returned in a single XMM register by both.
