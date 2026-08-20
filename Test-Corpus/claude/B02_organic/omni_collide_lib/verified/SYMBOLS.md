# SYMBOLS.md — Phase A symbol surface

Mechanically derived from `nm -D --defined-only` on both shared objects.

* C  `.so`: `c_src/build/libtranslated_rust.so`   (gcc 11.5.0, `cmake -DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
* Rust `.so`: `target/release/libomni_collide_lib.so` (`crate-type = ["cdylib"]`)

Reproduce with:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so   | awk '{print $3}' | sort > c_syms.txt
nm -D --defined-only target/release/libomni_collide_lib.so | awk '{print $3}' | sort > rust_syms.txt
comm -23 c_syms.txt rust_syms.txt      # must be EMPTY
```

## Build-time configuration surface

* `Cargo.toml` has **no `[features]` section** → exactly **one** valid feature
  combination (the empty/default one). `--no-default-features`,
  `--all-features` and the plain default build are all the same configuration.
* `c_src/CMakeLists.txt` defines no `option()`, no `target_compile_definitions`,
  and `grep -n '#if\|#ifdef\|#ifndef' c_src/src/lib.c c_src/include/lib.h`
  returns nothing → the C side likewise has exactly **one** configuration.

So Phase D's "repeat for every feature combination" collapses to the single
default configuration, which is verified twice anyway (Rust `.so` built with
`--release` **and** with the `dev` profile) — see `run_diff_tests.sh`.

## Result

* C exports **39** global symbols.
* Rust exports **39** global symbols.
* `comm -23 c_syms.txt rust_syms.txt` → **empty** (0 missing).
* `comm -13 c_syms.txt rust_syms.txt` → **empty** (0 extra).
* Rust `.so` undefined symbols are all libc / libgcc-unwind imports
  (`malloc`, `memcpy`, `_Unwind_*`, `__cxa_finalize`, …); there are
  **0 undefined non-libc symbols**.

## Symbol table

`c2GJKCache`, `c2Proxy`, `c2Simplex`, `c2sv` are internal types that appear in
the signatures below; all are `#[repr(C)]` in Rust with `const _: ()` layout
assertions (`size_of` 36 / 72 / 152 / 36) matching gcc.

| # | symbol | C binding | in C `.so` | in Rust `.so` | signature (C) |
|---|--------|-----------|------------|---------------|---------------|
|  1 | `c22`                 | T | yes | yes | `void c22(c2Simplex*)` |
|  2 | `c23`                 | T | yes | yes | `void c23(c2Simplex*)` |
|  3 | `c2AABBtoAABB`        | T | yes | yes | `int c2AABBtoAABB(c2AABB, c2AABB)` |
|  4 | `c2AABBtoCapsule`     | T | yes | yes | `int c2AABBtoCapsule(c2AABB, c2Capsule)` |
|  5 | `c2Add`               | T | yes | yes | `c2v c2Add(c2v, c2v)` |
|  6 | `c2BBVerts`           | T | yes | yes | `void c2BBVerts(c2v* out, c2AABB* bb)` |
|  7 | `c2CCW90`             | T | yes | yes | `c2v c2CCW90(c2v)` |
|  8 | `c2CapsuletoCapsule`  | T | yes | yes | `int c2CapsuletoCapsule(c2Capsule, c2Capsule)` |
|  9 | `c2CircletoAABB`      | T | yes | yes | `int c2CircletoAABB(c2Circle, c2AABB)` |
| 10 | `c2CircletoCapsule`   | T | yes | yes | `int c2CircletoCapsule(c2Circle, c2Capsule)` |
| 11 | `c2CircletoCircle`    | T | yes | yes | `int c2CircletoCircle(c2Circle, c2Circle)` |
| 12 | `c2Clampv`            | T | yes | yes | `c2v c2Clampv(c2v a, c2v lo, c2v hi)` |
| 13 | `c2Collided`          | T | yes | yes | `int c2Collided(const void*, C2_TYPE, const void*, C2_TYPE)` |
| 14 | `c2D`                 | T | yes | yes | `c2v c2D(c2Simplex*)` |
| 15 | `c2Det2`              | T | yes | yes | `float c2Det2(c2v, c2v)` |
| 16 | `c2Div`               | T | yes | yes | `c2v c2Div(c2v, float)` |
| 17 | `c2Dot`               | T | yes | yes | `float c2Dot(c2v, c2v)` |
| 18 | `c2GJK`               | T | yes | yes | `float c2GJK(const void*, C2_TYPE, const c2x*, const void*, C2_TYPE, const c2x*, c2v*, c2v*, int, int*, c2GJKCache*)` |
| 19 | `c2GJKSimplexMetric`  | T | yes | yes | `float c2GJKSimplexMetric(c2Simplex*)` |
| 20 | `c2L`                 | T | yes | yes | `c2v c2L(c2Simplex*)` |
| 21 | `c2Len`               | T | yes | yes | `float c2Len(c2v)` |
| 22 | `c2MakeProxy`         | T | yes | yes | `void c2MakeProxy(const void*, C2_TYPE, c2Proxy*)` |
| 23 | `c2Maxv`              | T | yes | yes | `c2v c2Maxv(c2v, c2v)` |
| 24 | `c2Minv`              | T | yes | yes | `c2v c2Minv(c2v, c2v)` |
| 25 | `c2Mulrv`             | T | yes | yes | `c2v c2Mulrv(c2r, c2v)` |
| 26 | `c2MulrvT`            | T | yes | yes | `c2v c2MulrvT(c2r, c2v)` |
| 27 | `c2Mulvs`             | T | yes | yes | `c2v c2Mulvs(c2v, float)` |
| 28 | `c2Mulxv`             | T | yes | yes | `c2v c2Mulxv(c2x, c2v)` |
| 29 | `c2Neg`               | T | yes | yes | `c2v c2Neg(c2v)` |
| 30 | `c2Norm`              | T | yes | yes | `c2v c2Norm(c2v)` |
| 31 | `c2RotIdentity`       | T | yes | yes | `c2r c2RotIdentity(void)` |
| 32 | `c2Skew`              | T | yes | yes | `c2v c2Skew(c2v)` |
| 33 | `c2Sub`               | T | yes | yes | `c2v c2Sub(c2v, c2v)` |
| 34 | `c2Support`           | T | yes | yes | `int c2Support(const c2v* verts, int count, c2v d)` |
| 35 | `c2V`                 | T | yes | yes | `c2v c2V(float, float)` |
| 36 | `c2Witness`           | T | yes | yes | `void c2Witness(c2Simplex*, c2v* a, c2v* b)` |
| 37 | `c2xIdentity`         | T | yes | yes | `c2x c2xIdentity(void)` |
| 38 | `omni_collide`        | T | yes | yes | `int omni_collide(C2_TYPE, float×5, C2_TYPE, float×5)` |
| 39 | `ptr_from_parts`      | T | yes | yes | `void* ptr_from_parts(C2_TYPE, float, float, float, float, float)` |

No symbol required translating a previously-skipped C module: `c_src/src/lib.c`
is the only C translation unit and every one of its non-`static` functions is
present in `src/lib.rs`.

## ABI notes (relevant to the differential tests)

System V AMD64 classification of the by-value aggregates, which the tests
exercise through real `extern "C"` function pointers:

| type | size | class | passed / returned in |
|------|------|-------|----------------------|
| `c2v`, `c2r`   | 8  | SSE            | one `xmm` (low 8 bytes) |
| `c2x`, `c2AABB`| 16 | SSE, SSE       | `xmm0` + `xmm1` |
| `c2Circle`     | 12 | SSE, SSE       | `xmm0` (8B) + `xmm1` (4B) |
| `c2Capsule`    | 20 | MEMORY         | on the stack |

`c2Capsule` being MEMORY class is why `c2AABBtoCapsule` /
`c2CapsuletoCapsule` / `c2CircletoCapsule` are worth testing separately from
the register-class pairs.
