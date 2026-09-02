# SYMBOLS.md — exported-symbol parity

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-4p7pmr.so \
  | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libcapsule_lib.so \
  | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort > /tmp/rust_syms.txt
comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt   # missing from Rust
comm -13 /tmp/c_syms.txt /tmp/rust_syms.txt   # extra in Rust
```

C `.so` defined symbols: **38**. Rust `.so` defined symbols: **38**.
Missing from Rust: **0**. Extra in Rust: **0**.

`c_src/src/lib.c` contains exactly 38 non-`static` function definitions and no
global variables, so the table below is the complete translation unit surface.
No C module was skipped: `src/lib.c` is the only source file listed in
`c_src/CMakeLists.txt`.

| # | symbol | C signature | in C `.so` | in Rust `.so` | status |
|---|--------|-------------|-----------|---------------|--------|
| 1 | `c2V` | `c2v c2V(float, float)` | T | T | OK |
| 2 | `c2Mulvs` | `c2v c2Mulvs(c2v, float)` | T | T | OK |
| 3 | `c2Maxv` | `c2v c2Maxv(c2v, c2v)` | T | T | OK |
| 4 | `c2Minv` | `c2v c2Minv(c2v, c2v)` | T | T | OK |
| 5 | `c2Clampv` | `c2v c2Clampv(c2v, c2v, c2v)` | T | T | OK |
| 6 | `c2Sub` | `c2v c2Sub(c2v, c2v)` | T | T | OK |
| 7 | `c2Dot` | `float c2Dot(c2v, c2v)` | T | T | OK |
| 8 | `c2RotIdentity` | `c2r c2RotIdentity(void)` | T | T | OK |
| 9 | `c2xIdentity` | `c2x c2xIdentity(void)` | T | T | OK |
| 10 | `c2BBVerts` | `void c2BBVerts(c2v*, c2AABB*)` | T | T | OK |
| 11 | `c2MakeProxy` | `void c2MakeProxy(const void*, C2_TYPE, c2Proxy*)` | T | T | OK |
| 12 | `c2Len` | `float c2Len(c2v)` | T | T | OK |
| 13 | `c2Det2` | `float c2Det2(c2v, c2v)` | T | T | OK |
| 14 | `c2GJKSimplexMetric` | `float c2GJKSimplexMetric(c2Simplex*)` | T | T | OK |
| 15 | `c2Mulrv` | `c2v c2Mulrv(c2r, c2v)` | T | T | OK |
| 16 | `c2Add` | `c2v c2Add(c2v, c2v)` | T | T | OK |
| 17 | `c2Mulxv` | `c2v c2Mulxv(c2x, c2v)` | T | T | OK |
| 18 | `c22` | `void c22(c2Simplex*)` | T | T | OK |
| 19 | `c23` | `void c23(c2Simplex*)` | T | T | OK |
| 20 | `c2Neg` | `c2v c2Neg(c2v)` | T | T | OK |
| 21 | `c2Skew` | `c2v c2Skew(c2v)` | T | T | OK |
| 22 | `c2CCW90` | `c2v c2CCW90(c2v)` | T | T | OK |
| 23 | `c2D` | `c2v c2D(c2Simplex*)` | T | T | OK |
| 24 | `c2Support` | `int c2Support(const c2v*, int, c2v)` | T | T | OK |
| 25 | `c2Witness` | `void c2Witness(c2Simplex*, c2v*, c2v*)` | T | T | OK |
| 26 | `c2Div` | `c2v c2Div(c2v, float)` | T | T | OK |
| 27 | `c2Norm` | `c2v c2Norm(c2v)` | T | T | OK |
| 28 | `c2L` | `c2v c2L(c2Simplex*)` | T | T | OK |
| 29 | `c2MulrvT` | `c2v c2MulrvT(c2r, c2v)` | T | T | OK |
| 30 | `c2GJK` | `float c2GJK(const void*, C2_TYPE, const c2x*, const void*, C2_TYPE, const c2x*, c2v*, c2v*, int, int*, c2GJKCache*)` | T | T | OK |
| 31 | `c2AABBtoAABB` | `int c2AABBtoAABB(c2AABB, c2AABB)` | T | T | OK |
| 32 | `c2AABBtoCapsule` | `int c2AABBtoCapsule(c2AABB, c2Capsule)` | T | T | OK |
| 33 | `c2CapsuletoCapsule` | `int c2CapsuletoCapsule(c2Capsule, c2Capsule)` | T | T | OK |
| 34 | `c2CircletoCircle` | `int c2CircletoCircle(c2Circle, c2Circle)` | T | T | OK |
| 35 | `c2CircletoAABB` | `int c2CircletoAABB(c2Circle, c2AABB)` | T | T | OK |
| 36 | `c2CircletoCapsule` | `int c2CircletoCapsule(c2Circle, c2Capsule)` | T | T | OK |
| 37 | `c2Collided` | `int c2Collided(const void*, C2_TYPE, const void*, C2_TYPE)` | T | T | OK |
| 38 | `capsule` | `int capsule(float, float, float, float, float)` | T | T | OK |

## Undefined (imported) symbols

C `.so` imports only `sqrtf@GLIBC` plus the standard CRT/ITM stubs
(`__cxa_finalize`, `__gmon_start__`, `_ITM_*`).

Rust `.so` imports only libc (`memcpy`, `malloc`, `free`, `abort`, …), the
unwinder (`_Unwind_*`) and the same CRT/ITM stubs. **0 undefined non-libc
symbols.** `sqrtf` is not imported because `f32::sqrt` lowers to the hardware
`sqrtss` instruction, which is IEEE-754 exactly-rounded — bit-identical to
glibc's `sqrtf`.

Automated check: `tests/symbol_parity.rs` (`symbol_parity_is_exact`) re-runs the
`nm -D` diff as part of `cargo test` and fails if either direction is non-empty.
