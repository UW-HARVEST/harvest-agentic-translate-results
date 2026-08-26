# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so

# Rust (only one build configuration exists — see "Feature combinations")
cargo build --no-default-features
# -> target/debug/libnext_double_lib.so
```

## Feature combinations

`Cargo.toml` declares **no `[features]` table** and `c_src/CMakeLists.txt`
declares **no `option()` / `target_compile_definitions` / `#ifdef` switches**
(the C source contains zero preprocessor conditionals — verified by
`grep -nE '#if|#ifdef|#ifndef' c_src/src/lib.c c_src/include/lib.h` → no hits).

Therefore the complete enumeration of valid build-time configurations is a
single combination: **the empty feature set**, which is simultaneously the
default set, `--no-default-features`, and `--all-features`.

| # | combination | `cargo check` | `cargo test` |
|---|-------------|---------------|--------------|
| 1 | `--no-default-features` (empty == default == all) | pass | pass |

## Exported (defined) dynamic symbols

`nm -D --defined-only c_src/build/libtranslated_rust.so`:

| symbol | type | present in Rust `.so`? |
|--------|------|------------------------|
| `next_double` | `T` | **yes** (`#[unsafe(no_mangle)] pub unsafe extern "C" fn next_double`) |

`nm -D --defined-only target/debug/libnext_double_lib.so`:

| symbol | type |
|--------|------|
| `next_double` | `T` |

### Symbol diff

```
C exported symbols not exported by Rust:  (none)
```

The diff is **empty**. Symbol parity is achieved.

### Symbols intentionally NOT exported

| C symbol | reason |
|----------|--------|
| `cn_rnd_next` | declared `static` in `c_src/src/lib.c`, so it has internal linkage and does not appear in `nm -D` of the C `.so`. It is reproduced in Rust as a private `fn cn_rnd_next` with identical semantics. Exporting it would *add* a symbol the C library does not have. |

`cn_rnd_t` is a type, not a symbol; it contributes no dynamic symbol in either
library. Its layout (`#[repr(C)] { state: [u64; 2] }`, 16 bytes, align 8) is
asserted by the differential tests via guard bytes around the struct.

## Undefined symbols in the Rust `.so`

All undefined symbols in `libnext_double_lib.so` are libc / libgcc-unwind
runtime imports pulled in by the Rust standard library, not missing
translation units:

* `libc.so.6`: `malloc`, `calloc`, `realloc`, `free`, `posix_memalign`,
  `memcpy`, `memmove`, `memset`, `bcmp`, `strlen`, `abort`, `getenv`,
  `getcwd`, `readlink`, `realpath`, `open64`, `close`, `read`, `write`,
  `writev`, `lseek64`, `stat64`, `fstat64`, `statx`, `mmap64`, `munmap`,
  `syscall`, `dl_iterate_phdr`, `__errno_location`, `__tls_get_addr`,
  `pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`,
  `gettid`, `__cxa_finalize`, `__cxa_thread_atexit_impl`
* `libgcc_s.so.1`: `_Unwind_*`
* weak/no-op: `_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable`,
  `__gmon_start__` (also present in the C `.so`)

**0 missing / undefined non-libc symbols.**
