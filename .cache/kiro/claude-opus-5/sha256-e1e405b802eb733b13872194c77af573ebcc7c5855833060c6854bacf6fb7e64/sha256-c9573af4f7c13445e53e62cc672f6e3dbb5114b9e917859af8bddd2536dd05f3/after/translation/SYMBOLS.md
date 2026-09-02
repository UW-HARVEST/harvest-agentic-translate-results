# SYMBOLS.md — public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C `.so`:    `c_src/build/libharvest-work-g6Fd0c.so`
- Rust `.so`: `translation/target/release/libaabb_lib.so`

Reproduce with:

```sh
nm -D --defined-only c_src/build/libharvest-work-g6Fd0c.so   | awk '$2=="T"{print $3}' | grep -v '^_' | sort > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libaabb_lib.so | awk '$2=="T"{print $3}' | grep -v '^_' | sort > /tmp/rs_syms.txt
comm -23 /tmp/c_syms.txt /tmp/rs_syms.txt   # missing from Rust  -> MUST be empty
comm -13 /tmp/c_syms.txt /tmp/rs_syms.txt   # extra in Rust      -> MUST be empty
```

`c_src/src/lib.c` declares nothing `static`, so every one of its 38 functions has
external linkage and appears in the dynamic symbol table. Only `aabb` is declared
in `include/lib.h`; the remaining 37 are still part of the ABI surface and are
therefore all covered here and by the differential tests.

## Parity table

| #  | symbol                | C `.so` | Rust `.so` | C signature (from `c_src/src/lib.c`) |
|----|-----------------------|---------|------------|--------------------------------------|
|  1 | `aabb`                | T       | T          | `int aabb(float,float,float,float)` |
|  2 | `c22`                 | T       | T          | `void c22(c2Simplex*)` |
|  3 | `c23`                 | T       | T          | `void c23(c2Simplex*)` |
|  4 | `c2AABBtoAABB`        | T       | T          | `int c2AABBtoAABB(c2AABB,c2AABB)` |
|  5 | `c2AABBtoCapsule`     | T       | T          | `int c2AABBtoCapsule(c2AABB,c2Capsule)` |
|  6 | `c2Add`               | T       | T          | `c2v c2Add(c2v,c2v)` |
|  7 | `c2BBVerts`           | T       | T          | `void c2BBVerts(c2v*,c2AABB*)` |
|  8 | `c2CCW90`             | T       | T          | `c2v c2CCW90(c2v)` |
|  9 | `c2CapsuletoCapsule`  | T       | T          | `int c2CapsuletoCapsule(c2Capsule,c2Capsule)` |
| 10 | `c2CircletoAABB`      | T       | T          | `int c2CircletoAABB(c2Circle,c2AABB)` |
| 11 | `c2CircletoCapsule`   | T       | T          | `int c2CircletoCapsule(c2Circle,c2Capsule)` |
| 12 | `c2CircletoCircle`    | T       | T          | `int c2CircletoCircle(c2Circle,c2Circle)` |
| 13 | `c2Clampv`            | T       | T          | `c2v c2Clampv(c2v,c2v,c2v)` |
| 14 | `c2Collided`          | T       | T          | `int c2Collided(const void*,C2_TYPE,const void*,C2_TYPE)` |
| 15 | `c2D`                 | T       | T          | `c2v c2D(c2Simplex*)` |
| 16 | `c2Det2`              | T       | T          | `float c2Det2(c2v,c2v)` |
| 17 | `c2Div`               | T       | T          | `c2v c2Div(c2v,float)` |
| 18 | `c2Dot`               | T       | T          | `float c2Dot(c2v,c2v)` |
| 19 | `c2GJK`               | T       | T          | `float c2GJK(const void*,C2_TYPE,const c2x*,const void*,C2_TYPE,const c2x*,c2v*,c2v*,int,int*,c2GJKCache*)` |
| 20 | `c2GJKSimplexMetric`  | T       | T          | `float c2GJKSimplexMetric(c2Simplex*)` |
| 21 | `c2L`                 | T       | T          | `c2v c2L(c2Simplex*)` |
| 22 | `c2Len`               | T       | T          | `float c2Len(c2v)` |
| 23 | `c2MakeProxy`         | T       | T          | `void c2MakeProxy(const void*,C2_TYPE,c2Proxy*)` |
| 24 | `c2Maxv`              | T       | T          | `c2v c2Maxv(c2v,c2v)` |
| 25 | `c2Minv`              | T       | T          | `c2v c2Minv(c2v,c2v)` |
| 26 | `c2Mulrv`             | T       | T          | `c2v c2Mulrv(c2r,c2v)` |
| 27 | `c2MulrvT`            | T       | T          | `c2v c2MulrvT(c2r,c2v)` |
| 28 | `c2Mulvs`             | T       | T          | `c2v c2Mulvs(c2v,float)` |
| 29 | `c2Mulxv`             | T       | T          | `c2v c2Mulxv(c2x,c2v)` |
| 30 | `c2Neg`               | T       | T          | `c2v c2Neg(c2v)` |
| 31 | `c2Norm`              | T       | T          | `c2v c2Norm(c2v)` |
| 32 | `c2RotIdentity`       | T       | T          | `c2r c2RotIdentity(void)` |
| 33 | `c2Skew`              | T       | T          | `c2v c2Skew(c2v)` |
| 34 | `c2Sub`               | T       | T          | `c2v c2Sub(c2v,c2v)` |
| 35 | `c2Support`           | T       | T          | `int c2Support(const c2v*,int,c2v)` |
| 36 | `c2V`                 | T       | T          | `c2v c2V(float,float)` |
| 37 | `c2Witness`           | T       | T          | `void c2Witness(c2Simplex*,c2v*,c2v*)` |
| 38 | `c2xIdentity`         | T       | T          | `c2x c2xIdentity(void)` |

## Result

```
C count: 38   Rust count: 38
missing from Rust: (none)
extra in Rust:    (none)
```

Undefined symbols in the Rust `.so` are libc / libgcc-unwind only
(`memcpy`, `malloc`, `abort`, `_Unwind_*`, …) — no unresolved project symbols.
The C `.so` additionally imports `sqrtf@GLIBC`; Rust lowers `f32::sqrt` to the
`sqrtss` instruction inline, which is the same IEEE-754 correctly-rounded
operation, so this import difference is not a behavioural difference.

## ABI notes verified by the tests

- `c2v` / `c2r` (2 × `float`, 8 bytes) return in the low half of `xmm0` and are
  passed in one SSE register. `#[repr(C)]` two-`f32` structs match.
- `c2x` (4 × `float`, 16 bytes) is classified SSE,SSE — `xmm0`/`xmm1`.
- `c2AABB` (16 bytes) and `c2Circle` (12 bytes) are passed in SSE registers;
  `c2Capsule` (20 bytes) exceeds 16 bytes and is passed on the stack (MEMORY).
- `c2Simplex` is `c2sv a,b,c,d; float div; int count`. `sizeof(c2sv) == 36`,
  align 4, so the Rust `[c2sv; 4]` field reproduces the C layout byte for byte
  (total 152 bytes) — this is what makes the C's `c2sv *verts = &s.a;` array
  walk legal to model as an array.
- No feature flags exist in `translation/Cargo.toml`, so the default build is
  the only feature combination (see `CONFIGS.md` § Feature combinations).
