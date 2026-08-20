# SYMBOLS.md — Symbol parity between the C `.so` and the Rust `.so`

Generated mechanically from:

```
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/release/libsiphash_lib.so
nm -D --defined-only target/debug/libsiphash_lib.so
```

## 1. Translation units in the C library

`c_src/CMakeLists.txt` compiles exactly one translation unit:

```
add_library(${project_name} SHARED
    src/lib.c)
```

`c_src/src/lib.c` defines exactly three functions (`grep -n -E '^[a-zA-Z_].*\(.*\)\s*\{' src/lib.c`):

| C source line | signature | linkage | must be exported? |
|---|---|---|---|
| 6 | `static size_t stbds_siphash_bytes(void *p, size_t len, size_t seed)` | `static` (internal) | **No** — `static` gives internal linkage |
| 110 | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` | external | **Yes** |
| 114 | `void siphash(int init)` | external (declared in `include/lib.h`) | **Yes** |

`c_src/include/lib.h` is a single line: `void siphash(int init);`

There is **no** untranslated C source: one `.c` file, all three of its functions are
present in `src/lib.rs` (`stbds_siphash_bytes` as a private Rust `fn`, the other two as
`#[unsafe(no_mangle)] pub extern "C"` wrappers). Nothing was stubbed.

## 2. Defined (exported) symbol table

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `siphash` | `T` | `T` | ✅ present in both |
| 2 | `stbds_hash_bytes` | `T` | `T` | ✅ present in both |

Raw output:

```
=== C .so defined ===
0000000000001547 T siphash
000000000000151a T stbds_hash_bytes

=== RUST .so defined ===
0000000000011d90 T siphash
0000000000011ef0 T stbds_hash_bytes
```

**Symbol diff (C-exported minus Rust-exported): EMPTY.**
**Extra symbols exported by Rust that C does not export: NONE.**

`stbds_siphash_bytes` is correctly absent from **both** `.so` files (it is `static` in C
and a private `fn` in Rust). Exporting it would be a parity *failure*, not a fix.

There are no macro-generated / mangled / versioned exported symbols in the C library, so
there is no additional generated-name surface to match.

## 3. Undefined symbols (imports)

The C `.so` imports only libc: `printf@GLIBC_2.2.5`, `puts@GLIBC_2.2.5` (gcc rewrites
`printf(" },\n")` into `puts(" },")`), plus the standard weak CRT symbols
(`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

The Rust `.so` imports `printf@GLIBC_2.2.5` (used deliberately, so the emitted bytes and
stdout buffering match the C build exactly) plus the normal Rust `std` / unwinder /
allocator set: `_Unwind_*`, `__errno_location`, `abort`, `bcmp`, `calloc`, `close`,
`dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `lseek64`, `malloc`, `memcpy`,
`memmove`, `memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`, `pthread_key_*`,
`pthread_setspecific`, `puts`, `read`, `readlink`, `realloc`, `realpath`, `stat64`,
`statx`, `strlen`, `syscall`, `write`, `writev`, `gettid`, `__tls_get_addr`,
`__cxa_thread_atexit_impl`.

**Every undefined symbol in the Rust `.so` is a libc / libgcc-unwind symbol resolved by
the dynamic loader. 0 missing or undefined non-libc symbols.**

Verified by `tests/symbol_parity.rs`, which re-runs `nm -D` on both objects at test time
and asserts the diff is empty (and that `stbds_siphash_bytes` is exported by neither).

## 4. Build configurations covered

`Cargo.toml` has **no `[features]` section**, so the complete set of valid feature
combinations is:

| # | invocation | resolved features |
|---|------------|-------------------|
| 1 | `cargo … ` (default) | ∅ |
| 2 | `cargo … --no-default-features` | ∅ |
| 3 | `cargo … --all-features` | ∅ |

All three are the *same* configuration (there is nothing to toggle); all three are run
by `./verify_all.sh` anyway. `c_src/CMakeLists.txt` likewise defines no options,
`#define`s or `#ifdef`-selected variants (`grep -nE '#ifdef|#if |#define' src/lib.c` →
no matches), so the C side has exactly one configuration too.
