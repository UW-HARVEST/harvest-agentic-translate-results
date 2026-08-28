# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libharvest-work-RF7L1c.so
nm -D --defined-only translation/target/release/libpoly_ray_lib.so
```

The C library is built from a single translation unit (`c_src/src/lib.c`, 402
lines) declared by `c_src/CMakeLists.txt` as `add_library(... SHARED src/lib.c)`.
There is exactly one C source file, so there is no possibility of an
un-translated module.

## Symbol table

`T` = defined text symbol in the dynamic table.
"Rust" column = present in `nm -D --defined-only` on `libpoly_ray_lib.so`.

| #  | symbol            | C  | Rust | C signature (from `src/lib.c` / `include/lib.h`)                                        |
|----|-------------------|----|------|------------------------------------------------------------------------------------------|
| 1  | `c2V`             | T  | T    | `c2v c2V(float x, float y)`                                                              |
| 2  | `c2Dot`           | T  | T    | `float c2Dot(c2v a, c2v b)`                                                              |
| 3  | `c2Len`           | T  | T    | `float c2Len(c2v a)`                                                                     |
| 4  | `c2Add`           | T  | T    | `c2v c2Add(c2v a, c2v b)`                                                                |
| 5  | `c2Sub`           | T  | T    | `c2v c2Sub(c2v a, c2v b)`                                                                |
| 6  | `c2Mulvs`         | T  | T    | `c2v c2Mulvs(c2v a, float b)`                                                            |
| 7  | `c2Div`           | T  | T    | `c2v c2Div(c2v a, float b)`                                                              |
| 8  | `c2Norm`          | T  | T    | `c2v c2Norm(c2v a)`                                                                      |
| 9  | `c2Minv`          | T  | T    | `c2v c2Minv(c2v a, c2v b)`                                                               |
| 10 | `c2Maxv`          | T  | T    | `c2v c2Maxv(c2v a, c2v b)`                                                               |
| 11 | `c2Skew`          | T  | T    | `c2v c2Skew(c2v a)`                                                                      |
| 12 | `c2Absv`          | T  | T    | `c2v c2Absv(c2v a)`                                                                      |
| 13 | `c2RaytoCircle`   | T  | T    | `int c2RaytoCircle(c2Ray A, c2Circle B, c2Raycast *out)`                                  |
| 14 | `c2AABBtoAABB`    | T  | T    | `int c2AABBtoAABB(c2AABB A, c2AABB B)`                                                    |
| 15 | `c2RaytoAABB`     | T  | T    | `int c2RaytoAABB(c2Ray A, c2AABB B, c2Raycast *out)`                                      |
| 16 | `c2CCW90`         | T  | T    | `c2v c2CCW90(c2v a)`                                                                     |
| 17 | `c2MulmvT`        | T  | T    | `c2v c2MulmvT(c2m a, c2v b)`                                                             |
| 18 | `c2AABBtoPoint`   | T  | T    | `int c2AABBtoPoint(c2AABB A, c2v B)`                                                      |
| 19 | `c2CircleToPoint` | T  | T    | `int c2CircleToPoint(c2Circle A, c2v B)`                                                  |
| 20 | `c2RaytoCapsule`  | T  | T    | `int c2RaytoCapsule(c2Ray A, c2Capsule B, c2Raycast *out)`                                |
| 21 | `c2RotIdentity`   | T  | T    | `c2r c2RotIdentity(void)`                                                                |
| 22 | `c2xIdentity`     | T  | T    | `c2x c2xIdentity(void)`                                                                  |
| 23 | `c2Mulrv`         | T  | T    | `c2v c2Mulrv(c2r a, c2v b)`                                                              |
| 24 | `c2MulrvT`        | T  | T    | `c2v c2MulrvT(c2r a, c2v b)`                                                             |
| 25 | `c2MulxvT`        | T  | T    | `c2v c2MulxvT(c2x a, c2v b)`                                                             |
| 26 | `c2RaytoPoly`     | T  | T    | `int c2RaytoPoly(c2Ray A, const c2Poly *B, const c2x *bx, c2Raycast *out)`                 |
| 27 | `c2CastRay`       | T  | T    | `int c2CastRay(c2Ray A, const void *B, const c2x *bx, C2_TYPE typeB, c2Raycast *out)`      |
| 28 | `poly_ray`        | T  | T    | `int poly_ray(c2Raycast *cast1, c2Raycast *cast2)`                                         |

**Counts:** C = 28 defined dynamic symbols, Rust = 28.
**`comm -23 c_syms rust_syms` (missing from Rust) = EMPTY.**
**`comm -13 c_syms rust_syms` (extra in Rust) = EMPTY.**

## Deliberately NOT exported (matches C)

These are `static inline` in the C and therefore have no dynamic symbol; the
Rust keeps them as private `#[inline] fn`:

| C symbol                                        | storage class   | Rust                                              |
|-------------------------------------------------|-----------------|---------------------------------------------------|
| `c2SignedDistPointToPlane_OneDimensional`       | `static inline` | private `fn` (no `#[no_mangle]`) — correct        |
| `c2RayToPlane_OneDimensional`                   | `static inline` | private `fn` (no `#[no_mangle]`) — correct        |

The C also open-codes `fabsf`/`fminf`/`fmaxf` as ternaries rather than calling
libm; the Rust mirrors this with private `c_abs`/`c_min`/`c_max` (NOT
`f32::abs`/`min`/`max`, whose NaN and `-0.0` semantics differ).

## Undefined-symbol check on the Rust `.so`

`nm -D --undefined-only translation/target/release/libpoly_ray_lib.so` yields
only libc / libgcc-unwind / weak-ITM entries:

`_ITM_*` (weak), `_Unwind_*` (libgcc_s), `__cxa_finalize` (weak),
`__cxa_thread_atexit_impl` (weak), `__errno_location`, `__gmon_start__` (weak),
`__tls_get_addr`, `abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`, `free`,
`fstat64`, `getcwd`, `getenv`, `gettid` (weak), `lseek64`, `malloc`, `memcpy`,
`memmove`, `memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`,
`pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`, `read`,
`readlink`, `realloc`, `realpath`, `stat64`, `statx` (weak), `strlen`,
`syscall`, `write`, `writev`.

`ldd` resolves everything against `libgcc_s.so.1`, `libc.so.6`,
`ld-linux-x86-64.so.2`.

**0 missing / undefined non-libc symbols.**  (The C `.so` correspondingly has
one undefined non-local symbol, `sqrtf`; the Rust lowers `f32::sqrt` to the
`sqrtss` instruction inline, which is the same operation, so no import is
needed.)

## Runtime dependency note (found while testing)

`c_src/CMakeLists.txt` never links `m`, so the C `.so` has an **undefined**
`sqrtf` and relies on the loading process to provide it:

```
$ readelf -d c_src/build/lib*.so | grep NEEDED
  (NEEDED)  Shared library: [libc.so.6]      # note: no libm
$ nm -D c_src/build/lib*.so | grep sqrtf
           U sqrtf
```

A *debug* Rust test binary happens to link `libm.so.6`, so `dlopen` of the C
library works by accident. An *optimised* one does not, and the C library then
fails to bind with `undefined symbol: sqrtf`. The harness therefore `dlopen`s
libm with `RTLD_GLOBAL` (and **leaks** the handle — dropping it would `dlclose`
and undo the fix) before loading either library, so both profiles behave the
same. Without this, half of the Phase D matrix would have been untestable.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, hence exactly one
feature combination exists (the default = empty set). `cargo check
--no-default-features` and `cargo check --all-features` are therefore identical
to `cargo check`; all three are nevertheless *exercised* (rather than assumed
equivalent) by `verify_all.sh`, which extracts the feature list from
`Cargo.toml` programmatically and runs the whole suite for each combination
against both a debug- and a release-built Rust `.so`.

## ABI notes verified by the tests

| type        | size | SysV AMD64 class            | exercised by                               |
|-------------|------|-----------------------------|--------------------------------------------|
| `c2v`       | 8    | SSE (one xmm, packed 2×f32) | every vector fn (arg + return)             |
| `c2r`       | 8    | SSE                         | `c2Mulrv`, `c2MulrvT`, `c2RotIdentity`     |
| `c2Circle`  | 12   | SSE, SSE                    | `c2RaytoCircle`, `c2CircleToPoint`         |
| `c2AABB`    | 16   | SSE, SSE                    | `c2AABBtoAABB`, `c2RaytoAABB`              |
| `c2m`       | 16   | SSE, SSE                    | `c2MulmvT`                                 |
| `c2x`       | 16   | SSE, SSE (arg *and* return) | `c2MulxvT`, `c2xIdentity`                  |
| `c2Capsule` | 20   | MEMORY (stack)              | `c2RaytoCapsule`                           |
| `c2Ray`     | 20   | MEMORY (stack)              | `c2Rayto*`, `c2CastRay`                    |
| `c2Poly`    | 132  | MEMORY, passed by pointer   | `c2RaytoPoly`, `c2CastRay`                 |
| `c2Raycast` | 12   | pointer out-param           | all raycasts                               |
