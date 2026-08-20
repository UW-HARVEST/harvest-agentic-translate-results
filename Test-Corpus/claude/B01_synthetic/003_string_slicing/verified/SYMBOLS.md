# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## What the C source is

`c_src/` contains exactly one translation unit, `c_src/src/main.c` (85 lines),
and `c_src/CMakeLists.txt` builds it as an **executable** (`add_executable(driver
src/main.c)`). The only function it defines is `main`; there are no headers, no
other modules, and therefore no other public API.

Because the verification harness compares *shared objects*, the same single
translation unit is additionally compiled as a shared library — without
touching anything in `c_src/`:

```sh
# C executable (as specified by c_src/CMakeLists.txt)
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/driver

# C shared library (same TU, PIC), written outside c_src/
gcc -shared -fPIC -O2 -o target/cdiff/libcdriver.so c_src/src/main.c
# -> target/cdiff/libcdriver.so
```

The Rust crate mirrors both shapes:

| artifact | C | Rust |
|---|---|---|
| executable | `c_src/build/driver` | `target/debug/driver` (`[[bin]] driver`) |
| shared object | `target/cdiff/libcdriver.so` | `target/debug/libdriver.so` (`[lib] crate-type = ["cdylib"]`) |

`src/imp.rs` holds the translation (`imp::c_main(argc, argv) -> c_int`, which
*returns* like the C `main` does and never calls `exit`), `src/lib.rs` exports
it as the C symbol `main`, and `src/main.rs` is the executable entry point.

## `nm -D` comparison (shared objects)

### Defined (exported) dynamic symbols

| symbol | C `libcdriver.so` | Rust `libdriver.so` |
|---|---|---|
| `main` | `T main` | `T main` |

```
$ nm -D --defined-only target/cdiff/libcdriver.so
0000000000001080 T main
$ nm -D --defined-only target/debug/libdriver.so
00000000000157c0 T main
```

**Symbol diff (C exports − Rust exports): EMPTY.** ✔

No symbol had to be added by stubbing: `main` is a real, complete translation
of the C `main`, and no C source file was left untranslated (there is only one
C file).

### Undefined (imported) dynamic symbols

The C `.so` imports `printf`, `puts`, `strlen`, `strtol` plus the usual
`__cxa_finalize` / `_ITM_*` / `__gmon_start__` weak stubs.

The Rust `.so` imports 50 symbols; **every one of them is libc / libgcc-unwind**
(`malloc`, `free`, `realloc`, `calloc`, `posix_memalign`, `memcpy`, `memmove`,
`memset`, `bcmp`, `strlen`, `write`, `writev`, `read`, `close`, `open64`,
`lseek64`, `stat64`, `fstat64`, `statx`, `mmap64`, `munmap`, `getcwd`, `getenv`,
`readlink`, `realpath`, `abort`, `syscall`, `gettid`, `dl_iterate_phdr`,
`__errno_location`, `__cxa_thread_atexit_impl`, `__cxa_finalize`,
`__tls_get_addr`, `pthread_key_create`, `pthread_key_delete`,
`pthread_setspecific`, `_Unwind_*`, `_ITM_*`, `__gmon_start__`) — i.e. the Rust
standard library's own runtime imports.

**0 missing / undefined non-libc symbols in the Rust `.so`.** ✔

Verify with:

```sh
diff <(nm -D --defined-only target/cdiff/libcdriver.so | awk '{print $NF}' | sort) \
     <(nm -D --defined-only target/debug/libdriver.so  | awk '{print $NF}' | sort)
```

(reproduced automatically by `tests/symbols.rs::c_exports_are_a_subset_of_rust_exports`)

## Build-time configurations (Phase A enumeration)

* `Cargo.toml` has **no `[features]` table** — the only dependency is the
  `libloading` dev-dependency used by the tests. The complete set of valid
  feature combinations is therefore exactly one: the default (empty) set.
  `cargo check --no-default-features` and `cargo check` are the same build.
* `c_src/CMakeLists.txt` declares no `option()`, no `add_definitions`, no
  `target_compile_definitions` and no `#ifdef` appears anywhere in
  `c_src/src/main.c`, so the C side likewise has exactly one configuration.
* Both the `dev` and the `release` profile (the latter sets `panic = "abort"`)
  are exercised; see `scripts/verify.sh`.

| # | configuration | `cargo check` | tests |
|---|---|---|---|
| 1 | default / `--no-default-features` (only combination that exists) | ✔ | ✔ |

## Notes on the crate layout

* `[lib] crate-type = ["cdylib"]`, `test = false`, `doctest = false`: the library
  exports a symbol literally named `main`, which would clash with the entry point
  that `rustc` generates for a test harness. The differential tests live in
  `tests/` and load the `.so` through `libloading`, so nothing is lost.
* `src/imp.rs` is included by both `src/lib.rs` and `src/main.rs` as a module
  (the binary does not link the `cdylib`), which keeps a single implementation
  without a duplicate `main` symbol at link time.
* See `VERIFICATION.md` for the full verification matrix and the list of
  divergences that were found and fixed.
