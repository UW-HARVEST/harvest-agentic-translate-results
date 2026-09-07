# SYMBOLS.md — exported-symbol parity

Derived mechanically from `nm -D` on both shared objects.

- C:    `c_src/build/libharvest-work-5ZGKZ6.so`
- Rust: `translation/target/release/libcolourblind_lib.so`

## C `.so` defined (`T`) symbols

| # | symbol | C binding | present in Rust `.so`? | notes |
|---|--------|-----------|------------------------|-------|
| 1 | `colourblind` | `T` (global text) | YES — `T colourblind` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn colourblind` in `src/lib.rs` |

## C `.so` weak / undefined symbols

These are toolchain/CRT artifacts, not library API. They are not part of the
translation surface but are listed for completeness.

| symbol | C | Rust | notes |
|--------|---|------|-------|
| `_ITM_deregisterTMCloneTable` | `w` | `w` | ELF boilerplate, both |
| `_ITM_registerTMCloneTable` | `w` | `w` | ELF boilerplate, both |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | `w` | ELF boilerplate, both |
| `__gmon_start__` | `w` | `w` | ELF boilerplate, both |

## Symbols `static` in C (deliberately NOT exported by either object)

`nm` on the C object file shows these as local (`t`); they are absent from
`nm -D` output of the `.so`. The Rust translation keeps them private too, so
the exported ABI matches exactly.

| C symbol | C linkage | Rust counterpart | Rust linkage |
|----------|-----------|------------------|--------------|
| `Protanopia`   | `static` (local `t`) | `protanopia`   | private `unsafe fn` |
| `Deuteranopia` | `static` (local `t`) | `deuteranopia` | private `unsafe fn` |
| `Tritanopia`   | `static` (local `t`) | `tritanopia`   | private `unsafe fn` |

## Extra symbols in the Rust `.so`

Every additional entry in the Rust `nm -D` output is an **undefined** (`U`) or
**weak-undefined** (`w`) import pulled in by the Rust standard library
(`libc`/`libgcc_s` unwinder), never a defined export:

`_Unwind_*` (10 × `U`), `__cxa_thread_atexit_impl` (`w`), `__errno_location`,
`__tls_get_addr`, `abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`, `free`,
`fstat64`, `getcwd`, `getenv`, `gettid` (`w`), `lseek64`, `malloc`, `memcpy`,
`memmove`, `memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`,
`pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`, `read`,
`readlink`, `realloc`, `realpath`, `stat64`, `statx` (`w`), `strlen`,
`syscall`, `write`, `writev`.

They add no API surface: the only defined text symbol the Rust `.so` exports is
`colourblind`, matching the C `.so`.

## Verdict

Symbol diff (defined non-libc exports, C → Rust): **EMPTY**.
No missing symbol, no untranslated C module. `src/lib.c` is the only C
translation unit in `CMakeLists.txt`, and all four of its functions
(`Protanopia`, `Deuteranopia`, `Tritanopia`, `colourblind`) are translated.

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.

Automated check: `translation/check_symbols.sh` (exits non-zero on any diff).
