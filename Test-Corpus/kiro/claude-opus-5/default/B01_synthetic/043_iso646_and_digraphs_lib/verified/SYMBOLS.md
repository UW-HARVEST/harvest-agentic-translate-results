# SYMBOLS.md — Phase A symbol map

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
nm -D -u  translation/target/release/libdriver.so
```

## C `.so` exported (defined, dynamic) symbols

```
0000000000001119 T driver
```

Weak/undefined entries in the C `.so` (not exports, listed for completeness):

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
U printf@GLIBC_2.2.5
U puts@GLIBC_2.2.5
```

## Rust `.so` exported (defined, dynamic) symbols

```
0000000000011700 T driver
```

## Parity table

| # | C symbol | type | present in Rust `.so`? | notes |
|---|----------|------|------------------------|-------|
| 1 | `driver` | `T` (global text) | YES — `T driver` | `#[unsafe(no_mangle)] pub extern "C" fn driver(x: c_int, y: c_int)` in `src/lib.rs` |

**Missing symbols: 0.** The whole C library is a single translation unit
(`c_src/src/driver.c`) declaring a single public function in
`c_src/include/driver.h`; there is no untranslated module, and no
`#[no_mangle]` wrapper needed to be added.

Extra symbols exported by Rust but not C: none (`driver` only). The Rust `.so`
is a `cdylib`, so Rust-internal symbols are not exported.

## Undefined (imported) symbols in the Rust `.so`

All are libc / unwinder / Rust-runtime imports, i.e. **0 missing or undefined
non-libc symbols**:

* libc: `printf`, `putchar`, `malloc`, `calloc`, `realloc`, `free`,
  `posix_memalign`, `memcpy`, `memmove`, `memset`, `bcmp`, `strlen`, `abort`,
  `__errno_location`, `getenv`, `getcwd`, `readlink`, `realpath`, `open64`,
  `close`, `read`, `write`, `writev`, `lseek64`, `stat64`, `fstat64`, `statx`,
  `mmap64`, `munmap`, `syscall`, `dl_iterate_phdr`, `gettid`,
  `pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`,
  `__cxa_thread_atexit_impl`, `__tls_get_addr`
* unwinder (`libgcc_s`): `_Unwind_*` (panic machinery of the `dev`/`test`
  profile; the `release` profile sets `panic = "abort"`)
* weak ELF boilerplate: `_ITM_*`, `__gmon_start__`, `__cxa_finalize`

Note: the Rust `.so` imports `printf` but not `puts`. LLVM's
`SimplifyLibCalls` rewrites the constant-argument call `puts("")` into
`putchar('\n')`, which writes the identical single byte through the identical
glibc `stdout` `FILE` buffer. This is an import-list difference only, not a
behavioural one, and it is verified byte-for-byte in Phase B.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section**, so the only build
configuration is the default one. Verified with:

```
grep -n '\[features\]' translation/Cargo.toml   # no match
cargo metadata --format-version 1 | ... features # {} for package `driver`
```
