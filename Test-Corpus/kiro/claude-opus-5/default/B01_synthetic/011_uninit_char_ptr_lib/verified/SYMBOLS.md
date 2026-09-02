# SYMBOLS.md — dynamic-symbol parity

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

The whole C library is one translation unit (`c_src/CMakeLists.txt` →
`add_library(driver SHARED src/driver.c)`), and `src/driver.c` defines exactly
four external functions. There are no macro-generated symbols, no namespace or
renaming macros, and no other C source files — so the complete C export surface
is these four names.

## Defined (exported) symbols

| # | C symbol | C type | Rust `.so` exports it? | Rust definition |
|---|----------|--------|------------------------|-----------------|
| 1 | `printLine` | `T` (text, global) | yes | `src/lib.rs` — `#[unsafe(no_mangle)] pub extern "C" fn printLine` |
| 2 | `bad`       | `T` | yes | `src/lib.rs` — `#[unsafe(no_mangle)] pub extern "C" fn bad` |
| 3 | `good`      | `T` | yes | `src/lib.rs` — `#[unsafe(no_mangle)] pub extern "C" fn good` |
| 4 | `driver`    | `T` | yes | `src/lib.rs` — `#[unsafe(naked)] #[unsafe(no_mangle)] pub extern "C" fn driver` |

On x86-64 all four are `#[unsafe(naked)]` transcriptions of the C build's
disassembly; `mod portable` carries `#[inline(never)]` equivalents for other
architectures. Both sets export the same four names.

**Missing from the Rust `.so`: none.** No module of the C source was skipped;
`src/driver.c` is translated in full. Nothing is stubbed or `unimplemented!()`.

Verified automatically by `tests/differential.rs::phase_d_symbol_parity`, which
shells out to `nm -D` on both objects and asserts the set difference
(C-defined minus Rust-defined) is empty.

## Undefined (imported) symbols

The C `.so` imports `puts@GLIBC_2.2.5` plus the usual weak toolchain symbols
(`__cxa_finalize`, `__gmon_start__`, `_ITM_*TMCloneTable`).

The Rust `.so` imports `puts@GLIBC_2.2.5` (the translation calls `puts`
directly, matching gcc's lowering of `printf("%s\n", line)`), the same weak
toolchain symbols, and additionally:

* glibc entry points pulled in by the Rust standard library:
  `abort bcmp calloc close dl_iterate_phdr free fstat64 getcwd getenv lseek64
  malloc memcpy memmove memset mmap64 munmap open64 posix_memalign
  pthread_key_create pthread_key_delete pthread_setspecific read readlink
  realloc realpath stat64 strlen syscall write writev __errno_location
  __tls_get_addr`, weak `gettid`, `statx`, `__cxa_thread_atexit_impl`;
* the libgcc unwinder ABI: `_Unwind_Backtrace _Unwind_GetDataRelBase
  _Unwind_GetIP _Unwind_GetIPInfo _Unwind_GetLanguageSpecificData
  _Unwind_GetRegionStart _Unwind_GetTextRelBase _Unwind_Resume _Unwind_SetGR
  _Unwind_SetIP`.

All of these resolve out of `libc.so.6` / `libgcc_s.so.1`, which are present on
any target that can run the C library. **0 missing/undefined non-libc symbols:**
`dlopen(..., RTLD_NOW)` on the Rust `.so` succeeds, which by definition means
every undefined symbol was bound eagerly — asserted by
`tests/differential.rs::phase_d_rtld_now_resolves_every_import`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
buildable configuration is the default (empty) feature set. `cargo test
--no-default-features` is therefore identical to `cargo test`; both are run by
`scripts/verify.sh`. There are no `#[cfg(feature = ...)]` sites in `src/lib.rs`
(`grep -c 'feature' src/lib.rs` → 0). The only `cfg` axis is
`target_arch = "x86_64"` vs. not, and this host is x86-64, so the naked-asm
`driver` is the code under test.
