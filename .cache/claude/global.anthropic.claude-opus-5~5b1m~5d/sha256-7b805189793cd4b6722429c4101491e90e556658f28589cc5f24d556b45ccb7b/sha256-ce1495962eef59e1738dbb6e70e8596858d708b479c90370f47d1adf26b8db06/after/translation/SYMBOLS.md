# SYMBOLS.md — Phase A surface map

Mechanically derived from:

```sh
# C reference
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libharvest-work-IbhHLG.so | awk '{print $3}' | sort

# Rust translation
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libgjk_lib.so | awk '{print $3}' | sort
```

`c_src` compiles exactly one translation unit (`src/lib.c`, see
`c_src/CMakeLists.txt`), so there is no "whole module never translated" case
here: all 31 dynamic symbols come from that single file and all 31 are present
in `src/lib.rs`.

## Symbol table

`T` = exported code symbol in `.dynsym`. Every row is exported by BOTH `.so`s.

| # | symbol | C signature (`c_src/src/lib.c`) | C | Rust |
|---|--------|---------------------------------|---|------|
| 1 | `c2V` | `c2v c2V(float x, float y)` | T | T |
| 2 | `c2Mulvs` | `c2v c2Mulvs(c2v a, float b)` | T | T |
| 3 | `c2Maxv` | `c2v c2Maxv(c2v a, c2v b)` | T | T |
| 4 | `c2Minv` | `c2v c2Minv(c2v a, c2v b)` | T | T |
| 5 | `c2Clampv` | `c2v c2Clampv(c2v a, c2v lo, c2v hi)` | T | T |
| 6 | `c2Sub` | `c2v c2Sub(c2v a, c2v b)` | T | T |
| 7 | `c2Dot` | `float c2Dot(c2v a, c2v b)` | T | T |
| 8 | `c2RotIdentity` | `c2r c2RotIdentity(void)` | T | T |
| 9 | `c2xIdentity` | `c2x c2xIdentity(void)` | T | T |
| 10 | `c2BBVerts` | `void c2BBVerts(c2v *out, c2AABB *bb)` | T | T |
| 11 | `c2MakeProxy` | `void c2MakeProxy(const void *shape, C2_TYPE type, c2Proxy *p)` | T | T |
| 12 | `c2Len` | `float c2Len(c2v a)` | T | T |
| 13 | `c2Det2` | `float c2Det2(c2v a, c2v b)` | T | T |
| 14 | `c2GJKSimplexMetric` | `float c2GJKSimplexMetric(c2Simplex *s)` | T | T |
| 15 | `c2Mulrv` | `c2v c2Mulrv(c2r a, c2v b)` | T | T |
| 16 | `c2Add` | `c2v c2Add(c2v a, c2v b)` | T | T |
| 17 | `c2Mulxv` | `c2v c2Mulxv(c2x a, c2v b)` | T | T |
| 18 | `c22` | `void c22(c2Simplex *s)` | T | T |
| 19 | `c23` | `void c23(c2Simplex *s)` | T | T |
| 20 | `c2Neg` | `c2v c2Neg(c2v a)` | T | T |
| 21 | `c2Skew` | `c2v c2Skew(c2v a)` | T | T |
| 22 | `c2CCW90` | `c2v c2CCW90(c2v a)` | T | T |
| 23 | `c2D` | `c2v c2D(c2Simplex *s)` | T | T |
| 24 | `c2Support` | `int c2Support(const c2v *verts, int count, c2v d)` | T | T |
| 25 | `c2Witness` | `void c2Witness(c2Simplex *s, c2v *a, c2v *b)` | T | T |
| 26 | `c2Div` | `c2v c2Div(c2v a, float b)` | T | T |
| 27 | `c2Norm` | `c2v c2Norm(c2v a)` | T | T |
| 28 | `c2L` | `c2v c2L(c2Simplex *s)` | T | T |
| 29 | `c2MulrvT` | `c2v c2MulrvT(c2r a, c2v b)` | T | T |
| 30 | `c2GJK` | `float c2GJK(const void*, C2_TYPE, const c2x*, const void*, C2_TYPE, const c2x*, c2v*, c2v*, int, int*, c2GJKCache*)` | T | T |
| 31 | `gjk` | `void gjk(char, c2v*, c2v*, float×9)` — the only symbol in `include/lib.h` | T | T |

## Diff result

```
comm -23 c_syms.txt rust_syms.txt   # in C, missing from Rust
  (empty)
comm -13 c_syms.txt rust_syms.txt   # in Rust, not in C
  (empty)
```

**0 missing symbols. 0 extra symbols. 31 == 31.**

Verified automatically by `tests/symbols.rs::symbol_parity_c_vs_rust`, which
re-runs `nm -D` on both `.so`s and fails on any asymmetry, so the parity claim
cannot silently rot.

## Undefined (imported) symbols in the Rust `.so`

`nm -D --undefined-only libgjk_lib.so` lists 49 entries. Every one is libc,
libgcc's unwinder, or a linker-provided weak hook — i.e. the standard Rust
`cdylib` runtime imports, not un-translated library code:

* glibc: `malloc`, `calloc`, `realloc`, `free`, `posix_memalign`, `memcpy`,
  `memmove`, `memset`, `bcmp`, `strlen`, `abort`, `getenv`, `getcwd`,
  `readlink`, `realpath`, `open64`, `close`, `read`, `write`, `writev`,
  `lseek64`, `stat64`, `fstat64`, `statx`, `mmap64`, `munmap`, `syscall`,
  `gettid`, `dl_iterate_phdr`, `__errno_location`, `__cxa_finalize`,
  `__cxa_thread_atexit_impl`, `__tls_get_addr`, `pthread_key_create`,
  `pthread_key_delete`, `pthread_setspecific`
* libgcc unwinder (panic machinery): `_Unwind_*` (11 symbols)
* weak/optional hooks: `_ITM_registerTMCloneTable`,
  `_ITM_deregisterTMCloneTable`, `__gmon_start__`

**0 undefined non-libc symbols.** The C `.so` correspondingly imports only
`sqrtf@GLIBC_2.2.5` plus the same weak hooks.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so `cargo`
exposes exactly one feature configuration (the empty default). There is nothing
to cross-product; `--no-default-features` and the default build are the same
build. `./check_features.sh` enumerates the feature list from `Cargo.toml` and
runs `cargo check` + the full test suite for every combination it finds (which
is the single empty one), so a future `[features]` addition is covered
automatically.
