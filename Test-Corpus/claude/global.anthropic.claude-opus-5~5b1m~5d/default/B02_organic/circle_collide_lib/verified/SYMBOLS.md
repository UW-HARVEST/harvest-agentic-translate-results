# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libharvest-work-ws7ccv.so

# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libcircle_collide_lib.so
```

## C source inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

```
add_library(${project_name} SHARED src/lib.c)
```

`find c_src -name '*.c' -o -name '*.h'` →
`c_src/src/lib.c` (144 lines), `c_src/include/lib.h` (1 line).

Both are translated in `translation/src/lib.rs`. **No C module was skipped**, so
no "absent implementation" case from the Phase A rule applies here — every
missing symbol (had there been one) would have been an export-wrapper problem
only. In fact there are no missing symbols at all (see the diff below).

## Symbol table

`T` = global text symbol. All 12 are `extern "C"` + `#[no_mangle]` in Rust.

| # | symbol | C `.so` | Rust `.so` | C signature (`c_src/src/lib.c`) | Rust item |
|---|--------|---------|------------|----------------------------------|-----------|
| 1 | `c2V`               | T | T | `c2v c2V(float, float)`                 | `pub extern "C" fn c2V` |
| 2 | `c2Mulvs`           | T | T | `c2v c2Mulvs(c2v, float)`               | `pub extern "C" fn c2Mulvs` |
| 3 | `c2Maxv`            | T | T | `c2v c2Maxv(c2v, c2v)`                  | `pub extern "C" fn c2Maxv` |
| 4 | `c2Minv`            | T | T | `c2v c2Minv(c2v, c2v)`                  | `pub extern "C" fn c2Minv` |
| 5 | `c2Clampv`          | T | T | `c2v c2Clampv(c2v, c2v, c2v)`           | `pub extern "C" fn c2Clampv` |
| 6 | `c2Sub`             | T | T | `c2v c2Sub(c2v, c2v)`                   | `pub extern "C" fn c2Sub` |
| 7 | `c2Dot`             | T | T | `float c2Dot(c2v, c2v)`                 | `pub extern "C" fn c2Dot` |
| 8 | `c2CircletoCircle`  | T | T | `int c2CircletoCircle(c2Circle, c2Circle)` | `pub extern "C" fn c2CircletoCircle` |
| 9 | `c2CircletoAABB`    | T | T | `int c2CircletoAABB(c2Circle, c2AABB)`  | `pub extern "C" fn c2CircletoAABB` |
| 10 | `c2CircletoCapsule` | T | T | `int c2CircletoCapsule(c2Circle, c2Capsule)` | `pub extern "C" fn c2CircletoCapsule` |
| 11 | `c2Collided`        | T | T | `int c2Collided(const void*, const void*, C2_TYPE)` | `pub unsafe extern "C" fn c2Collided` |
| 12 | `circle_collide`    | T | T | `int circle_collide(float, float, float)` | `pub extern "C" fn circle_collide` |

`c_src/include/lib.h` declares only `circle_collide`; the other 11 symbols have
external linkage in `lib.c` (no `static`) and are therefore part of the C `.so`'s
public ABI, so the Rust `.so` must export them too.

## Diff

```
$ diff <(nm -D --defined-only c_src/build/libharvest-work-ws7ccv.so   | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libcircle_collide_lib.so | awk '{print $3}' | sort)
(no output)
```

- Symbols in C but missing from Rust: **0**
- Undefined non-libc symbols in the Rust `.so`: **0**
  `nm -D --undefined-only translation/target/release/libcircle_collide_lib.so`
  lists only libc / libgcc_s / ld.so imports pulled in by `core`+`std`'s panic
  and backtrace machinery, i.e. all of them resolve against the system runtime:
  `_ITM_{de,}registerTMCloneTable`, `_Unwind_*@GCC_*`, `__cxa_finalize`,
  `__cxa_thread_atexit_impl`, `__errno_location`, `__gmon_start__`,
  `__tls_get_addr`, `abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`,
  `free`, `fstat64`, `getcwd`, `getenv`, `gettid`, `lseek64`, `malloc`,
  `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`,
  `posix_memalign`, `pthread_key_{create,delete}`, `pthread_setspecific`,
  `read`, `readlink`, `realloc`, `realpath`, `stat64`, `statx`, `strlen`,
  `syscall`, `write`, `writev`.
  Verified resolvable end-to-end: `dlopen()` of the Rust `.so` from the
  integration tests succeeds and every one of the 12 symbols is callable.

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the only build
configuration is the default one. `cargo check --no-default-features` and
`cargo check` are the same build; the automated sweep in
`translation/check_features.sh` confirms the symbol diff is empty for every
(i.e. the single) feature combination.

## Result

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`.
- [x] `nm -D` shows 0 undefined non-libc symbols in the Rust `.so`.
