# SYMBOLS.md — Phase A surface map

Mechanically derived from:

```
nm -D --defined-only c_src/build/libharvest-work-9oPgJg.so | awk '{print $3}' | sort -u
nm -D --defined-only translation/target/release/libomni_collide_lib.so | awk '{print $3}' | sort -u
```

The C library is built by `c_src/CMakeLists.txt` from the single translation
unit `c_src/src/lib.c`; the library base name is derived from the *parent*
directory name (`cmake_path(GET parent FILENAME project_name)`), so it is
`libharvest-work-9oPgJg.so` in this checkout. Tests glob `c_src/build/lib*.so`
instead of hard-coding it.

The Rust `.so` is `translation/target/{debug,release}/libomni_collide_lib.so`
(`crate-type = ["cdylib"]`, `[lib] name = "omni_collide_lib"`).

## Symbol table (39 symbols exported by the C `.so`)

Legend for "Rust": `exported` = present in `nm -D` of the Rust cdylib with the
exact same name.

| #  | C symbol             | C signature (from `c_src/src/lib.c`)                                                                                                   | Rust      |
|----|----------------------|----------------------------------------------------------------------------------------------------------------------------------------|-----------|
| 1  | `c2V`                | `c2v c2V(float x, float y)`                                                                                                            | exported  |
| 2  | `c2Mulvs`            | `c2v c2Mulvs(c2v a, float b)`                                                                                                          | exported  |
| 3  | `c2Maxv`             | `c2v c2Maxv(c2v a, c2v b)`                                                                                                             | exported  |
| 4  | `c2Minv`             | `c2v c2Minv(c2v a, c2v b)`                                                                                                             | exported  |
| 5  | `c2Clampv`           | `c2v c2Clampv(c2v a, c2v lo, c2v hi)`                                                                                                  | exported  |
| 6  | `c2Sub`              | `c2v c2Sub(c2v a, c2v b)`                                                                                                              | exported  |
| 7  | `c2Dot`              | `float c2Dot(c2v a, c2v b)`                                                                                                            | exported  |
| 8  | `c2RotIdentity`      | `c2r c2RotIdentity(void)`                                                                                                              | exported  |
| 9  | `c2xIdentity`        | `c2x c2xIdentity(void)`                                                                                                                | exported  |
| 10 | `c2BBVerts`          | `void c2BBVerts(c2v *out, c2AABB *bb)`                                                                                                 | exported  |
| 11 | `c2MakeProxy`        | `void c2MakeProxy(const void *shape, C2_TYPE type, c2Proxy *p)`                                                                        | exported  |
| 12 | `c2Len`              | `float c2Len(c2v a)`                                                                                                                   | exported  |
| 13 | `c2Det2`             | `float c2Det2(c2v a, c2v b)`                                                                                                           | exported  |
| 14 | `c2GJKSimplexMetric` | `float c2GJKSimplexMetric(c2Simplex *s)`                                                                                               | exported  |
| 15 | `c2Mulrv`            | `c2v c2Mulrv(c2r a, c2v b)`                                                                                                            | exported  |
| 16 | `c2Add`              | `c2v c2Add(c2v a, c2v b)`                                                                                                              | exported  |
| 17 | `c2Mulxv`            | `c2v c2Mulxv(c2x a, c2v b)`                                                                                                            | exported  |
| 18 | `c22`                | `void c22(c2Simplex *s)`                                                                                                               | exported  |
| 19 | `c23`                | `void c23(c2Simplex *s)`                                                                                                               | exported  |
| 20 | `c2Neg`              | `c2v c2Neg(c2v a)`                                                                                                                     | exported  |
| 21 | `c2Skew`             | `c2v c2Skew(c2v a)`                                                                                                                    | exported  |
| 22 | `c2CCW90`            | `c2v c2CCW90(c2v a)`                                                                                                                   | exported  |
| 23 | `c2D`                | `c2v c2D(c2Simplex *s)`                                                                                                                | exported  |
| 24 | `c2Support`          | `int c2Support(const c2v *verts, int count, c2v d)`                                                                                    | exported  |
| 25 | `c2Witness`          | `void c2Witness(c2Simplex *s, c2v *a, c2v *b)`                                                                                         | exported  |
| 26 | `c2Div`              | `c2v c2Div(c2v a, float b)`                                                                                                            | exported  |
| 27 | `c2Norm`             | `c2v c2Norm(c2v a)`                                                                                                                    | exported  |
| 28 | `c2L`                | `c2v c2L(c2Simplex *s)`                                                                                                                | exported  |
| 29 | `c2MulrvT`           | `c2v c2MulrvT(c2r a, c2v b)`                                                                                                           | exported  |
| 30 | `c2GJK`              | `float c2GJK(const void *A, C2_TYPE typeA, const c2x *ax, const void *B, C2_TYPE typeB, const c2x *bx, c2v *outA, c2v *outB, int use_radius, int *iterations, c2GJKCache *cache)` | exported  |
| 31 | `c2AABBtoAABB`       | `int c2AABBtoAABB(c2AABB A, c2AABB B)`                                                                                                 | exported  |
| 32 | `c2AABBtoCapsule`    | `int c2AABBtoCapsule(c2AABB A, c2Capsule B)`                                                                                           | exported  |
| 33 | `c2CapsuletoCapsule` | `int c2CapsuletoCapsule(c2Capsule A, c2Capsule B)`                                                                                     | exported  |
| 34 | `c2CircletoCircle`   | `int c2CircletoCircle(c2Circle A, c2Circle B)`                                                                                         | exported  |
| 35 | `c2CircletoAABB`     | `int c2CircletoAABB(c2Circle A, c2AABB B)`                                                                                             | exported  |
| 36 | `c2CircletoCapsule`  | `int c2CircletoCapsule(c2Circle A, c2Capsule B)`                                                                                       | exported  |
| 37 | `c2Collided`         | `int c2Collided(const void *A, C2_TYPE typeA, const void *B, C2_TYPE typeB)`                                                           | exported  |
| 38 | `ptr_from_parts`     | `void *ptr_from_parts(C2_TYPE typ, float a, float b, float c, float d, float e)`                                                       | exported  |
| 39 | `omni_collide`       | `int omni_collide(C2_TYPE, float,float,float,float,float, C2_TYPE, float,float,float,float,float)`                                     | exported  |

## Diff result

```
$ comm -23 c_syms.txt rust_syms.txt   # missing from Rust
(empty)
$ comm -13 c_syms.txt rust_syms.txt   # extra in Rust
(empty)
```

**0 missing symbols. 0 extra symbols.** No C module was skipped: `lib.c` is the
only translation unit and every one of its 39 non-`static` functions has a
`#[unsafe(no_mangle)] pub extern "C"` counterpart in `translation/src/lib.rs`.

Undefined (imported) symbols are libc / libgcc only on both sides:

```
$ nm -D --undefined-only c_src/build/libharvest-work-9oPgJg.so
  w _ITM_deregisterTMCloneTable   w _ITM_registerTMCloneTable
  w __cxa_finalize@GLIBC_2.2.5    w __gmon_start__
  U malloc@GLIBC_2.2.5            U sqrtf@GLIBC_2.2.5

$ nm -D --undefined-only translation/target/release/libomni_collide_lib.so
  malloc, calloc, realloc, free, posix_memalign, memcpy, memmove, memset, bcmp,
  strlen, abort, getenv, getcwd, realpath, readlink, open64, close, read, write,
  writev, lseek64, stat64, fstat64, statx, mmap64, munmap, syscall,
  dl_iterate_phdr, __errno_location, __tls_get_addr, pthread_key_create,
  pthread_key_delete, pthread_setspecific, gettid, __cxa_finalize,
  __cxa_thread_atexit_impl, __gmon_start__, _ITM_*, _Unwind_*
```

Everything past `malloc` in the Rust list is the standard `std` runtime
(allocator shim, panic/backtrace machinery, TLS). The only *functional* import
either library makes is `malloc`, used by `ptr_from_parts`; the C additionally
imports `sqrtf` from `libm`, whereas the Rust build lowers `f32::sqrt` to a
`sqrtss` instruction. These are *imports*, not exports, so they do not affect
export parity — and `verify.sh` step 3 asserts the Rust `.so` imports nothing
outside that libc/libgcc allowlist.

## ABI notes verified for the differential harness (x86-64 SysV)

| type        | size | class                          | how passed / returned          |
|-------------|------|--------------------------------|--------------------------------|
| `c2v`       | 8    | SSE                            | 1 xmm reg; returned in xmm0    |
| `c2r`       | 8    | SSE                            | 1 xmm reg; returned in xmm0    |
| `c2x`       | 16   | SSE, SSE                       | 2 xmm regs; returned xmm0/xmm1 |
| `c2Circle`  | 12   | SSE, SSE                       | 2 xmm regs                     |
| `c2AABB`    | 16   | SSE, SSE                       | 2 xmm regs                     |
| `c2Capsule` | 20   | MEMORY (>16 bytes)             | on the stack                   |
| `C2_TYPE`   | 4    | INTEGER (`int`-sized C enum)   | 1 gp reg; accepts *any* `int`  |

`C2_TYPE` is modelled in Rust as `pub type C2_TYPE = c_int` with `const`s, not
as a Rust `enum`, precisely so that out-of-range values crossing the FFI
boundary are well-defined (see `ERRORS.md`).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default one. The completion gate's "every feature
combination" therefore reduces to: default features, verified in both the
`debug` and `release` profiles (the harness loads *both* Rust `.so`s when
present, so opt-level-3 floating-point differences would be caught too).

`verify.sh` still enumerates the `[features]` table programmatically and loops
over the full power set, so it keeps working if features are ever added. With
none declared it runs the suite three times: `<default>`,
`--no-default-features` and `--all-features`.

## C-side completeness

`c_src` has exactly one translation unit and it declares **no** `static`
functions, no `inline` functions, no macros that expand to definitions and no
`#ifdef` blocks:

```
$ grep -cE '^[A-Za-z_][A-Za-z0-9_ ]*\*?[A-Za-z0-9_]+\(' c_src/src/lib.c
39
$ grep -cE '\bstatic\b|\binline\b|^#if|^#ifdef' c_src/src/lib.c
0
```

39 definitions, 39 exports, 39 Rust `#[unsafe(no_mangle)] pub extern "C"`
wrappers. Nothing was skipped, and `translation/src/lib.rs` contains no
`unimplemented!()`, `todo!()`, `unreachable!()` or `panic!()` — every symbol has
a real translated body:

```
$ grep -cE 'unimplemented!|todo!|unreachable!|panic!' translation/src/lib.rs
0
```

## How this file was checked

```
$ ./verify.sh
== 3. symbol parity (nm -D)
ok   release: all 39 C symbols exported by the Rust .so
ok   debug:   all 39 C symbols exported by the Rust .so
ok   all undefined symbols in the Rust .so are libc/libgcc
```
