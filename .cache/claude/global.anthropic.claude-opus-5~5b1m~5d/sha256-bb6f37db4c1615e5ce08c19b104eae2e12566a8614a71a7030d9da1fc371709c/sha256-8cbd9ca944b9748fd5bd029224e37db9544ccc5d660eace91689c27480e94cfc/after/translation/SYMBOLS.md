# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C translation units in scope

The whole library is `c_src/src/driver.c` (the only source file listed in
`c_src/CMakeLists.txt`) plus the single public header `c_src/include/driver.h`.
There are no other modules, so there is no un-translated C source.

`c_src/CMakeLists.txt` declares only `add_library(driver SHARED src/driver.c)` —
there is **no** `add_executable` and no `main()` anywhere in `c_src`, so the
project builds **no binary driver**. The "compare C and Rust stdout of the
binary" clause of the completion gate is therefore not applicable (there is
nothing to run); stdout is instead compared through the FFI boundary, because
the one public function's entire observable effect *is* what it prints.

## Exported (defined, dynamic) symbols

`nm -D --defined-only`:

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `driver` | `T driver` | `T driver` | `void driver(int)`; declared in `driver.h`, defined in `driver.c`. Rust: `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver(x: c_int)` |

Exported-symbol diff (C minus Rust): **empty**.
Exported-symbol diff (Rust minus C): **empty**.

There are no macro-generated exports, no exported data objects, and no
additional public entry points hidden in the source: `driver.h` declares
exactly one function, and `driver.c` defines exactly that one function with
external linkage.

## Undefined (imported) symbols

The C `.so` imports one non-weak libc symbol:

```
U printf@GLIBC_2.2.5
```

The Rust `.so` imports the same `printf@GLIBC_2.2.5` (the translation calls
libc `printf` directly so that formatting and stdout buffering are identical),
plus the usual Rust `std`/`libgcc` runtime imports, all of which are libc or
unwinder symbols:

`_Unwind_*` (GCC unwinder), `__cxa_finalize`, `__cxa_thread_atexit_impl`,
`__errno_location`, `__tls_get_addr`, `abort`, `bcmp`, `calloc`, `close`,
`dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `gettid`, `lseek64`,
`malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`,
`posix_memalign`, `printf`, `pthread_key_create`, `pthread_key_delete`,
`pthread_setspecific`, `read`, `readlink`, `realloc`, `realpath`, `stat64`,
`statx`, `strlen`, `syscall`, `write`, `writev`.

**0 missing symbols; 0 undefined non-libc / non-unwinder symbols in the Rust
`.so`.** Symbol parity holds.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one (`--no-default-features` and the default build
are the same build). The whole gate is therefore satisfied by the single
default configuration; this is verified explicitly by the
`features_surface_is_only_default` check in `tests/differential.rs` and by
running the suite under `--no-default-features` as well.
