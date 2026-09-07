# Phase A.1 — Symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` — `nm -D c_src/build/libdriver.so`

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
0000000000001109 T driver
                 U printf@GLIBC_2.2.5
```

`nm -D --defined-only` on the C `.so` yields exactly one strong defined symbol:

| # | C symbol | type | source of definition | exported by Rust `.so`? |
|---|----------|------|----------------------|-------------------------|
| 1 | `driver` | `T` (global text) | `c_src/src/driver.c:28`, declared `c_src/include/driver.h:26` | YES — `#[unsafe(no_mangle)] pub extern "C" fn driver` in `translation/src/lib.rs` |

The four `w` (weak) entries — `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__` — are toolchain
/ CRT-injected weak references present in every glibc shared object, not part
of the library's API. The Rust `.so` carries the same weak set (plus
`__cxa_thread_atexit_impl`, `gettid`, `statx` from Rust's std). `printf` is an
undefined import in both.

## Rust `.so` — `nm -D --defined-only translation/target/release/libdriver.so`

```
00000000000116c0 T driver
```

Both Rust profiles are built and both are loaded by the differential suite:

* `target/release/libdriver.so` — `opt-level=3`, `panic = "abort"`.
* `target/debug/libdriver.so` — `opt-level=0`, **overflow checks ON**,
  `panic = "unwind"`. This profile is not produced by `cargo test` for a
  `crate-type = ["cdylib"]` package, so it must be built explicitly with
  `cargo build`. It matters: it is the configuration in which non-wrapping
  arithmetic would trap instead of silently wrapping. Verified empirically —
  replacing `wrapping_mul(2)` with `* 2` in a throwaway copy makes the dev
  profile `.so` abort with `attempt to multiply with overflow` on
  `driver(INT_MAX)` (exit 134), whereas the real translation prints `298`,
  matching the C. Both profiles export the same single symbol:

```
$ nm -D --defined-only translation/target/debug/libdriver.so
00000000000118b0 T driver
```

## Symbol diff

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $NF}' | sort) \
           <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
(empty)
```

**Missing from Rust: none.** No C source file was left untranslated: the C
library is a single translation unit (`src/driver.c`) with a single public
header (`include/driver.h`), and `add_library(driver SHARED src/driver.c)` in
`c_src/CMakeLists.txt` confirms there is no second module. No macro-generated or
namespace-prefixed symbols exist (the header contains no renaming macros).

There are no `static`/internal-linkage helper functions in the C source, so
there is nothing hidden behind the dynamic-symbol table either.

## Undefined non-libc symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only glibc
(`printf`, `malloc`, `memcpy`, `write`, `pthread_key_create`, …) and
libgcc unwinder (`_Unwind_*`) imports, all of which resolve at load time. There
are **0 missing/undefined non-libc symbols**.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section** and no optional
dependencies, so `--no-default-features` and the default build are the same
single configuration. There is exactly one feature combination to verify.
`c_src` likewise contains no `#ifdef`-gated compile-time configuration (the only
preprocessor conditional in the whole C tree is the `DRIVER_H_` include guard).

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(driver SHARED ...)` — there is
no `add_executable`, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. The project builds **no driver
binary**, so the "compare C and Rust binary stdout" item of the completion gate
is not applicable. Equivalent coverage is obtained by capturing the process
stdout produced through each `.so` (see `tests/differential.rs`).
