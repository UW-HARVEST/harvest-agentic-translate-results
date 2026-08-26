# SYMBOLS.md — Phase A: symbol surface

Derived mechanically from `nm -D` on both shared libraries.

* C  `.so`: `c_src/build/libdriver.so`  (built with
  `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust `.so`: `target/release/libdriver.so` (built with `cargo build --release`)

Regenerate / re-verify with `./check_symbols.sh`.

## Translation-unit inventory (completeness check)

Every C source file compiled into the C `.so` must have a Rust counterpart.
`c_src/CMakeLists.txt` compiles exactly one source file:

| C translation unit | compiled into `.so` by CMake | Rust counterpart | status |
|--------------------|------------------------------|------------------|--------|
| `c_src/src/driver.c`     | yes (`add_library(driver SHARED src/driver.c)`) | `src/driver.rs` | translated |
| `c_src/include/driver.h` | header only (declares `driver`)                | `src/driver.rs` | translated |

No C source file is missing a Rust translation, so no Phase-A "translate the
missing module" work is required.

## Exported (defined, global) symbols

`nm -D --defined-only <so> | awk '$2=="T" || $2=="D" || $2=="B"'`

| # | symbol | C `.so` type | Rust `.so` type | declared in header? | notes |
|---|--------|--------------|-----------------|---------------------|-------|
| 1 | `driver`    | `T` | `T` | yes (`driver.h:27`) | public entry point |
| 2 | `fma_array` | `T` | `T` | **no** — but non-`static`, so it is an exported ABI symbol of the C `.so` | low-level entry point; must be exported by Rust too, and is (`#[no_mangle]` in `src/driver.rs`) |

**Missing from the Rust `.so`: none.** The symbol diff is empty.

## Deliberately NOT exported

| C symbol | reason |
|----------|--------|
| `inner`  | declared `static void inner(int*, int)` in `driver.c`, so it has internal linkage and does not appear in `nm -D` of the C `.so`. `src/driver.rs` mirrors this with a private `unsafe fn inner` (no `#[no_mangle]`). Exporting it would be a *divergence*, not a fix. |

## Undefined / imported symbols

The C `.so` imports only two non-weak symbols, both libc:

```
U memcpy@GLIBC_2.14
U printf@GLIBC_2.2.5
```

The Rust `.so` imports those same two symbols (`src/driver.rs` deliberately
calls libc `printf`/`memcpy` through `extern "C"` so that formatting and
stdout buffering are byte-identical) plus the usual Rust-runtime imports, all
of which resolve against `libc.so.6` / `libgcc_s.so.1`:

```
libgcc_s.so.1 unwinder: _Unwind_*            (panic/backtrace machinery)
libc:  abort bcmp calloc close dl_iterate_phdr free fstat64 getcwd getenv
       lseek64 malloc memcpy memmove memset mmap64 munmap open64
       posix_memalign printf pthread_key_create pthread_key_delete
       pthread_setspecific read readlink realloc realpath stat64 strlen
       syscall write writev __errno_location __tls_get_addr
weak:  _ITM_registerTMCloneTable _ITM_deregisterTMCloneTable __cxa_finalize
       __cxa_thread_atexit_impl __gmon_start__ gettid statx
```

`ldd target/release/libdriver.so` reports no missing objects, so there are
**0 missing/undefined non-libc symbols**.

## Gate

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with
      the exact same name (`driver`, `fma_array`).
- [x] No extra public symbols are exported by the Rust `.so` (in particular
      `inner` stays private, matching the C `static`).
- [x] `nm -D` shows 0 missing/undefined non-libc symbols in the Rust `.so`.
