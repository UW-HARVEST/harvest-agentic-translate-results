# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Source of truth:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only build/libharvest-work-pfRjrv.so
# Rust
cd translation && cargo build --release --offline
nm -D --defined-only target/release/libaabb_lib.so
```

The C shared object is `c_src/build/lib<parent-dir-name>.so` (CMake derives the
project name from the parent directory name, see `c_src/CMakeLists.txt`), i.e.
`libharvest-work-pfRjrv.so` in this checkout. The Rust shared object is
`translation/target/{debug,release}/libaabb_lib.so` (`[lib] name = "aabb_lib"`,
`crate-type = ["cdylib"]`).

`c_src/src/lib.c` is a single translation unit with **no** `static` functions, so
**every** function it defines has external linkage and appears in the C `.so`'s
dynamic symbol table. `c_src/include/lib.h` only declares `aabb`, but the other
37 functions are still part of the ABI surface and are therefore all verified.

## Symbol table (38 symbols)

| # | symbol | C type | in Rust `.so` | Rust item | C signature (from `c_src/src/lib.c`) |
|---|--------|--------|---------------|-----------|--------------------------------------|
| 1 | `aabb` | `T` | yes | `aabb` | `int aabb(float,float,float,float)` |
| 2 | `c22` | `T` | yes | `c22` | `void c22(c2Simplex*)` |
| 3 | `c23` | `T` | yes | `c23` | `void c23(c2Simplex*)` |
| 4 | `c2AABBtoAABB` | `T` | yes | `c2AABBtoAABB` | `int c2AABBtoAABB(c2AABB,c2AABB)` |
| 5 | `c2AABBtoCapsule` | `T` | yes | `c2AABBtoCapsule` | `int c2AABBtoCapsule(c2AABB,c2Capsule)` |
| 6 | `c2Add` | `T` | yes | `c2Add` | `c2v c2Add(c2v,c2v)` |
| 7 | `c2BBVerts` | `T` | yes | `c2BBVerts` | `void c2BBVerts(c2v*,c2AABB*)` |
| 8 | `c2CCW90` | `T` | yes | `c2CCW90` | `c2v c2CCW90(c2v)` |
| 9 | `c2CapsuletoCapsule` | `T` | yes | `c2CapsuletoCapsule` | `int c2CapsuletoCapsule(c2Capsule,c2Capsule)` |
| 10 | `c2CircletoAABB` | `T` | yes | `c2CircletoAABB` | `int c2CircletoAABB(c2Circle,c2AABB)` |
| 11 | `c2CircletoCapsule` | `T` | yes | `c2CircletoCapsule` | `int c2CircletoCapsule(c2Circle,c2Capsule)` |
| 12 | `c2CircletoCircle` | `T` | yes | `c2CircletoCircle` | `int c2CircletoCircle(c2Circle,c2Circle)` |
| 13 | `c2Clampv` | `T` | yes | `c2Clampv` | `c2v c2Clampv(c2v,c2v,c2v)` |
| 14 | `c2Collided` | `T` | yes | `c2Collided` | `int c2Collided(const void*,C2_TYPE,const void*,C2_TYPE)` |
| 15 | `c2D` | `T` | yes | `c2D` | `c2v c2D(c2Simplex*)` |
| 16 | `c2Det2` | `T` | yes | `c2Det2` | `float c2Det2(c2v,c2v)` |
| 17 | `c2Div` | `T` | yes | `c2Div` | `c2v c2Div(c2v,float)` |
| 18 | `c2Dot` | `T` | yes | `c2Dot` | `float c2Dot(c2v,c2v)` |
| 19 | `c2GJK` | `T` | yes | `c2GJK` | `float c2GJK(const void*,C2_TYPE,const c2x*,const void*,C2_TYPE,const c2x*,c2v*,c2v*,int,int*,c2GJKCache*)` |
| 20 | `c2GJKSimplexMetric` | `T` | yes | `c2GJKSimplexMetric` | `float c2GJKSimplexMetric(c2Simplex*)` |
| 21 | `c2L` | `T` | yes | `c2L` | `c2v c2L(c2Simplex*)` |
| 22 | `c2Len` | `T` | yes | `c2Len` | `float c2Len(c2v)` |
| 23 | `c2MakeProxy` | `T` | yes | `c2MakeProxy` | `void c2MakeProxy(const void*,C2_TYPE,c2Proxy*)` |
| 24 | `c2Maxv` | `T` | yes | `c2Maxv` | `c2v c2Maxv(c2v,c2v)` |
| 25 | `c2Minv` | `T` | yes | `c2Minv` | `c2v c2Minv(c2v,c2v)` |
| 26 | `c2Mulrv` | `T` | yes | `c2Mulrv` | `c2v c2Mulrv(c2r,c2v)` |
| 27 | `c2MulrvT` | `T` | yes | `c2MulrvT` | `c2v c2MulrvT(c2r,c2v)` |
| 28 | `c2Mulvs` | `T` | yes | `c2Mulvs` | `c2v c2Mulvs(c2v,float)` |
| 29 | `c2Mulxv` | `T` | yes | `c2Mulxv` | `c2v c2Mulxv(c2x,c2v)` |
| 30 | `c2Neg` | `T` | yes | `c2Neg` | `c2v c2Neg(c2v)` |
| 31 | `c2Norm` | `T` | yes | `c2Norm` | `c2v c2Norm(c2v)` |
| 32 | `c2RotIdentity` | `T` | yes | `c2RotIdentity` | `c2r c2RotIdentity(void)` |
| 33 | `c2Skew` | `T` | yes | `c2Skew` | `c2v c2Skew(c2v)` |
| 34 | `c2Sub` | `T` | yes | `c2Sub` | `c2v c2Sub(c2v,c2v)` |
| 35 | `c2Support` | `T` | yes | `c2Support` | `int c2Support(const c2v*,int,c2v)` |
| 36 | `c2V` | `T` | yes | `c2V` | `c2v c2V(float,float)` |
| 37 | `c2Witness` | `T` | yes | `c2Witness` | `void c2Witness(c2Simplex*,c2v*,c2v*)` |
| 38 | `c2xIdentity` | `T` | yes | `c2xIdentity` | `c2x c2xIdentity(void)` |

## Diff result

```
$ comm -23 <(nm -D --defined-only c_src/build/libharvest-work-pfRjrv.so | awk '{print $3}' | sort -u) \
           <(nm -D --defined-only translation/target/release/libaabb_lib.so | awk '{print $3}' | sort -u)
<empty>
```

**0 missing symbols.** No module of `c_src` was skipped: `src/lib.c` is the only
C source file and every one of its 38 external definitions has a real Rust
implementation (no stubs, no `unimplemented!()`).

## Undefined symbols in the Rust `.so`

`nm -D -u target/release/libaabb_lib.so` lists only libc / libgcc-unwind /
`ld.so` imports (`memcpy`, `malloc`, `abort`, `_Unwind_*`, `dl_iterate_phdr`,
…) pulled in by the Rust standard library. **0 undefined non-libc symbols.**

The C `.so` imports `sqrtf` from `libm`; the Rust build lowers `f32::sqrt` to
the `sqrtss` instruction so no import is required (bit-identical for all
inputs — `sqrtf` is correctly rounded and `glibc` uses `sqrtss` too).

## Non-function symbols

The C `.so` exports no data symbols (no globals in `lib.c`), so there is nothing
else to mirror. The `C2_TYPE` enumerators are compile-time constants in C and are
mirrored as `pub const C2_TYPE_* : c_int` in Rust (no linker symbol in either).

## Types crossing the FFI boundary

Layout is asserted at compile time in `src/lib.rs` (`const _: () = { … }`) and
re-verified at runtime by the differential tests, which pass these structs
by value / by pointer through both `.so`s:

| type | size | align | notes |
|------|------|-------|-------|
| `c2v` | 8 | 4 | SSE class → `xmm` register when passed by value |
| `c2r` | 8 | 4 | SSE class |
| `c2x` | 16 | 4 | SSE,SSE → 2 `xmm` registers |
| `c2Circle` | 12 | 4 | SSE,SSE (second eightbyte is `r` + tail padding) |
| `c2AABB` | 16 | 4 | SSE,SSE |
| `c2Capsule` | 20 | 4 | > 16 bytes → MEMORY class (stack) |
| `c2GJKCache` | 36 | 4 | always by pointer |
| `c2Proxy` | 72 | 4 | always by pointer |
| `c2sv` | 36 | 4 | element of `c2Simplex` |
| `c2Simplex` | 152 | 4 | `{a,b,c,d}` modelled as `[c2sv; 4]`; the C code itself aliases `&s->a` as an array (`c2sv *verts = &s.a;`) |
