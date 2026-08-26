# SYMBOLS.md — Phase A: symbol surface

Derived mechanically from `nm -D` on the built shared objects.

## Build commands used

```sh
# C reference
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so

# Rust translation (cdylib, see [lib] in Cargo.toml)
cargo build --release --no-default-features
# -> target/release/libhsl_to_rgb_lib.so
```

## C source inventory (completeness check)

The *entire* C library is:

| file | lines | contents |
|------|-------|----------|
| `c_src/include/lib.h` | 1 | `void hsl_to_rgb(float *dest, const float *src);` |
| `c_src/src/lib.c` | 48 | the single definition of `hsl_to_rgb` |

`c_src/CMakeLists.txt` compiles exactly `src/lib.c` into one `SHARED` library and
links `m` (libm, used for `fmodf`). There is no second module, no conditional
compilation, no generated code, and no macro that synthesises extra symbols, so
the public surface is a single function. Nothing was skipped by the translation.

## Exported (defined, dynamic) symbols

`nm -D --defined-only`, filtered to global symbols:

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `hsl_to_rgb` | `T` (0x1109) | `T` | **present in both** |

Symbol diff (`C exported` minus `Rust exported`): **empty**.

Rust also exports no *extra* global symbols (the only `T` entry in
`nm -D --defined-only` is `hsl_to_rgb`), so the ABI surface is an exact match.

## Undefined / imported symbols

| `.so` | undefined symbols |
|-------|-------------------|
| C | `fmodf@GLIBC_2.2.5`; weak: `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__` |
| Rust | `__errno_location`, `abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `lseek64`, `malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`, `pthread_key_*`, `pthread_setspecific`, `read`, `readlink`, `realloc`, `realpath`, `stat64`, `strlen`, `syscall`, `write`, `writev`, `fmodf`, `_Unwind_*`; weak: `_ITM_*`, `__cxa_finalize`, `__cxa_thread_atexit_impl`, `__gmon_start__`, `gettid`, `statx` |

All Rust undefined symbols are libc / libm / libgcc-unwind (the Rust `std`
runtime + panic machinery); **0 missing/undefined non-libc symbols**.

### `fmodf` must come from libm, not from `compiler_builtins`

The C code calls glibc's `fmodf` (`U fmodf@GLIBC_2.2.5`). A naive translation
using Rust's `%` operator on `f32` links `compiler_builtins::math::libm_math::fmod::fmodf`
*statically* into the `.so` (visible as a **local** `t fmodf`), which is a
different implementation and is free to return a different NaN bit pattern for
the special cases (`fmodf(±inf, 2.0)`).

`src/lib.rs` therefore declares

```rust
unsafe extern "C" { fn fmodf(x: c_float, y: c_float) -> c_float; }
```

so the Rust `.so` imports the *same* `fmodf@GLIBC_2.2.5` the C `.so` uses. This
is verified mechanically by `tests/differential.rs::symbols::*`:

* `symbol_parity_c_exports_are_all_exported_by_rust` — the diff is empty,
* `rust_so_imports_fmodf_from_libm` — `fmodf` appears as an **undefined**
  dynamic symbol (imported), and there is no local definition shadowing it.

## Checklist

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
- [x] Every symbol exported by the C `.so` is exported by the Rust `.so` under
      the exact same name.
- [x] No C source file/module was left untranslated.
