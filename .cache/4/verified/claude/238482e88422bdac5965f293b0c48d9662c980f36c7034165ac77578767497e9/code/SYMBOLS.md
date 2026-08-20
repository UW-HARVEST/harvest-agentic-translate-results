# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C   `.so`: `c_src/build/libtranslated_rust.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust `.so`: `target/debug/libmd5_digest_lib.so` (`crate-type = ["cdylib"]`)

## Translation-unit inventory (completeness check)

The whole C library is one translation unit and one public header, so there is
no possibility of a "skipped module":

| C file | lines | translated in |
|--------|-------|---------------|
| `c_src/src/lib.c` | 21 | `src/lib.rs` (`md5_digest`) |
| `c_src/include/lib.h` | 14 | `src/lib.rs` (`tflac_u8`, `tflac_u32`, `struct tflac_md5`) |

`c_src/CMakeLists.txt` compiles exactly `src/lib.c` into one `SHARED` library.
No other sources exist, so every function defined in the C library is accounted
for below.

## Symbol table

| # | C symbol (`nm -D`) | type | exported by Rust `.so` | notes |
|---|--------------------|------|------------------------|-------|
| 1 | `md5_digest` | `T` (global text) | YES — `T md5_digest` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn md5_digest` in `src/lib.rs` |

There are no macro-generated symbols (the header contains no function-like
macros and no namespacing/renaming macros), no exported data symbols, and no
exported globals/statics on either side.

Types (`tflac_u8`, `tflac_u32`, `struct tflac_md5`) are compile-time only and
produce no linker symbols; the Rust mirrors are `#[repr(C)]`/type aliases and
are verified for size/alignment (16 / 4) by unit tests.

## Diff result

```
$ comm -23 <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $NF}' | sort) \
           <(nm -D --defined-only target/debug/libmd5_digest_lib.so  | awk '{print $NF}' | sort)
(empty)
```

* Symbols exported by C `.so`: **1**
* Symbols exported by C `.so` that the Rust `.so` is missing: **0**
* Undefined (`nm -D -u`) symbols in the Rust `.so`: all are libc/libgcc-unwind
  imports pulled in by the Rust standard library
  (`memcpy`, `malloc`, `free`, `abort`, `_Unwind_*`, `pthread_*`, `mmap64`, ...).
  **0** non-libc/non-runtime undefined symbols.

Symbol parity: **PASS** (verified again in Phase D, see `VERIFICATION.md`).
