# SYMBOLS.md — symbol parity between the C `.so` and the Rust `.so`

## Build commands used

```
# C
cd translated_rust/c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> translated_rust/c_src/build/libdriver.so

# Rust
cd translated_rust && cargo build            # (== --no-default-features, no features exist)
# -> translated_rust/target/debug/libdriver.so
```

## C source surface

`c_src/include/lib.h` (the whole public header):

```c
char *custom_strdup(const char *str);
```

`c_src/src/lib.c` defines exactly one function (`custom_strdup`); there are no
other translation units in `CMakeLists.txt` (`add_library(driver SHARED src/lib.c)`),
no `static` helpers, no global/exported data objects, and no macro-generated
symbols. Therefore the complete expected export set is a single symbol.

## `nm -D --defined-only` comparison

| # | symbol | type | in C `.so` | in Rust `.so` | status |
|---|--------|------|-----------|---------------|--------|
| 1 | `custom_strdup` | `T` (global text) | yes | yes | MATCH |

Raw output:

```
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001129 T custom_strdup

$ nm -D --defined-only target/debug/libdriver.so
0000000000011e40 T custom_strdup
```

### Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $NF}' | sort) \
       <(nm -D --defined-only target/debug/libdriver.so | awk '{print $NF}' | sort)
(no output — diff is EMPTY)
```

**0 symbols exported by the C `.so` are missing from the Rust `.so`.**
No `#[no_mangle]` wrapper had to be added and no C module was found untranslated:
`lib.c` is the only C source file and its only function is exported by the Rust
crate as `#[unsafe(no_mangle)] pub unsafe extern "C" fn custom_strdup`.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only target/debug/libdriver.so` lists only libc / libgcc
runtime imports (all resolvable from the platform's `libc.so.6` /
`libgcc_s.so.1`), i.e. **0 missing/undefined non-libc symbols**:

* libc: `malloc`, `memcpy`, `strlen`, `free`, `calloc`, `realloc`,
  `posix_memalign`, `memmove`, `memset`, `bcmp`, `abort`, `getenv`, `getcwd`,
  `readlink`, `realpath`, `open64`, `close`, `read`, `write`, `writev`,
  `lseek64`, `stat64`, `fstat64`, `statx` (weak), `mmap64`, `munmap`,
  `dl_iterate_phdr`, `syscall`, `gettid` (weak), `__errno_location`,
  `__tls_get_addr`, `__cxa_finalize` (weak), `__cxa_thread_atexit_impl` (weak),
  `pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`
* libgcc unwinder (Rust std panic/backtrace machinery): `_Unwind_*`
* toolchain weak hooks: `_ITM_registerTMCloneTable`,
  `_ITM_deregisterTMCloneTable`, `__gmon_start__`

Of these, only `malloc`, `memcpy` and `strlen` are used by the translated
function itself (matching the C original's `malloc`/`memcpy`/`strlen`); the rest
come from the Rust standard library that is linked into every `cdylib`.

## Build-time configuration surface

* `Cargo.toml` has **no `[features]` section** ⇒ the only feature combinations
  that exist are the empty set (`--no-default-features`) and the default set
  (also empty). Both were checked and both build and test identically.
* `c_src/CMakeLists.txt` declares **no `option()`/`add_definitions()`** and the
  C source contains **no `#ifdef`/`#if`** conditional compilation ⇒ one single
  C configuration.

| # | configuration | `cargo check` | `cargo test` |
|---|---------------|---------------|--------------|
| 1 | (default — no features) | PASS | PASS |
| 2 | `--no-default-features` (identical set) | PASS | PASS |
