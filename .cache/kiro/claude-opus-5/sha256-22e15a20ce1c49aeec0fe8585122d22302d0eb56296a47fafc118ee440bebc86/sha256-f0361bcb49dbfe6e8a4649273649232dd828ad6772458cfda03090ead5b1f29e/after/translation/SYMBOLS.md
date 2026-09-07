# SYMBOLS.md — Symbol parity between C `.so` and Rust `.so`

Generated mechanically from:

```
nm -D --defined-only  c_src/build/libdriver.so
nm -D --defined-only  translation/target/release/libdriver.so
nm -D --undefined-only <each>
```

Target: `x86_64-unknown-linux-gnu` (SysV ABI, `char` is **signed**).

## C source inventory

`c_src/CMakeLists.txt` declares exactly one translation unit:

```cmake
add_library(driver SHARED
    src/driver.c)
```

There is **no** `add_executable` — the project builds a shared library only, so
there is no driver binary whose stdout needs comparing (Phase B binary check is
N/A, recorded in the completion gate).

`c_src/src/driver.c` defines exactly two functions, both `extern` (no `static`),
both returning `void`:

| C definition | file:line |
|---|---|
| `void printHexCharLine (char charHex)` | `src/driver.c:28` |
| `void driver(char data)` | `src/driver.c:33` |

`c_src/include/driver.h` declares only `void driver(char data);`.
`printHexCharLine` is **not** declared in the public header but *is* an exported
dynamic symbol of the `.so`, so it is part of the ABI surface and is verified
here and in Phases B/C.

There are no macro-generated symbols (no function-defining macros anywhere in
`c_src`), no global/static data symbols, and no `#ifdef` build variants.

## Exported (defined, dynamic) symbols

| # | Symbol | C `.so` | Rust `.so` | Status |
|---|--------|---------|------------|--------|
| 1 | `driver`           | `T` | `T` | ✅ present in both |
| 2 | `printHexCharLine` | `T` | `T` | ✅ present in both |

Raw output:

```
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001143 T driver
0000000000001119 T printHexCharLine

$ nm -D --defined-only translation/target/release/libdriver.so | grep -vE ' (rust_|__|_ITM|_fini|_init)'
0000000000011710 T driver
0000000000011730 T printHexCharLine
```

The Rust `.so` additionally exports Rust-runtime symbols
(`rust_eh_personality`, `__rust_alloc`, `_init`, `_fini`, …). Extra symbols are
allowed by the gate; the gate requires that no **C** symbol is missing from Rust.

### Symbol diff

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $3}' | sort -u) \
           <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort -u)
(empty)
```

**MISSING FROM RUST: none.** No `#[no_mangle]` wrapper had to be added and no
untranslated C module was found — `src/driver.c` is the whole library and both of
its functions are translated in `translation/src/lib.rs`. No stubs, no
`unimplemented!()`.

## Undefined (imported) symbols

The C `.so` imports one non-weak libc symbol: `printf@GLIBC_2.2.5`.

The Rust `.so` imports `printf@GLIBC_2.2.5` (it deliberately delegates to libc
`printf` so formatting and stdout buffering are byte-identical) plus the usual
Rust `std`/`libunwind` dependencies: `_Unwind_*@GCC_*`, `malloc`, `calloc`,
`realloc`, `free`, `posix_memalign`, `memcpy`, `memmove`, `memset`, `bcmp`,
`strlen`, `abort`, `getenv`, `getcwd`, `readlink`, `realpath`, `open64`,
`close`, `read`, `write`, `writev`, `lseek64`, `stat64`, `fstat64`, `mmap64`,
`munmap`, `dl_iterate_phdr`, `syscall`, `__errno_location`, `__tls_get_addr`,
`pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`, and the weak
`statx`, `gettid`, `__cxa_finalize`, `__cxa_thread_atexit_impl`,
`__gmon_start__`, `_ITM_*`.

**Non-libc / non-runtime undefined symbols in the Rust `.so`: 0.** Every import
resolves from `libc.so.6` / `libgcc_s.so.1`, so the Rust `.so` loads and runs
standalone (confirmed: `libloading::Library::new` on it succeeds in the tests).

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section** and no optional
dependencies, therefore the only build configuration is the default one
(equivalent to `--no-default-features`, which is also verified). This is
re-checked mechanically by `scripts/verify.sh`.
