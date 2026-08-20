# SYMBOLS.md — Phase A: public ABI surface

Derived mechanically from `nm -D` on the C shared library and the Rust cdylib.

## Build commands used

```sh
# C reference library (default configuration — CMAKE_BUILD_TYPE empty, i.e. no -O)
cd translated_rust/c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so

# Rust translation
cargo build --release            # -> target/release/libdiv_euclid_lib.so
cargo build                      # -> target/debug/libdiv_euclid_lib.so
```

## Public headers scanned

`c_src/include/lib.h` declares exactly one entry point and contains **no**
namespace-renaming macros (`#define foo PREFIX_foo`), so the source-level name
is also the final linker name:

```c
int div_euclid(int v1, int v2);
```

`c_src/src/lib.c` defines exactly that one function and no other
externally-visible object (no globals, no file-static-turned-extern, no
macro-generated symbols).

## `nm -D --defined-only` — C `.so` (the definition of done)

| # | symbol | type | also exported by Rust `.so`? | Rust definition |
|---|--------|------|------------------------------|-----------------|
| 1 | `div_euclid` | `T` (global text) | **YES** (`T div_euclid`) | `src/lib.rs`, `#[unsafe(no_mangle)] pub extern "C" fn div_euclid(v1: c_int, v2: c_int) -> c_int` |

Weak/compiler-generated entries present in the C `.so` and NOT part of the
library's ABI (they are emitted by the toolchain into every shared object and
are also present in the Rust `.so`):

| symbol | C | Rust |
|--------|---|------|
| `_ITM_deregisterTMCloneTable` | `w` (undefined weak) | `w` |
| `_ITM_registerTMCloneTable` | `w` (undefined weak) | `w` |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | `w` |
| `__gmon_start__` | `w` | `w` |

## Symbol diff

```
$ comm -23 <(nm -D --defined-only C.so   | awk '{print $NF}' | sort) \
           <(nm -D --defined-only RUST.so | awk '{print $NF}' | sort)
(empty)
```

**Missing symbols: 0.** No C source file was left untranslated
(`c_src/src/lib.c` is the only translation unit in `c_src/CMakeLists.txt`, and
its single function is implemented in `src/lib.rs`). No stubs / no
`unimplemented!()` were used.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only target/release/libdiv_euclid_lib.so` lists only libc /
platform-runtime imports pulled in by the Rust standard library:

* glibc: `malloc`, `calloc`, `realloc`, `free`, `posix_memalign`, `memcpy`,
  `memmove`, `memset`, `bcmp`, `strlen`, `open64`, `close`, `read`, `write`,
  `writev`, `lseek64`, `readlink`, `realpath`, `getcwd`, `getenv`, `stat64`,
  `fstat64`, `statx`, `mmap64`, `munmap`, `abort`, `syscall`,
  `__errno_location`, `dl_iterate_phdr`, `pthread_key_create`,
  `pthread_key_delete`, `pthread_setspecific`, `__cxa_thread_atexit_impl`,
  `__tls_get_addr`, `gettid`
* libgcc unwinder: `_Unwind_Backtrace`, `_Unwind_Resume`, `_Unwind_Get*`,
  `_Unwind_Set*`

**Non-libc / non-runtime undefined symbols: 0.**

## Verification

`tests/phase_d_symbols.rs` re-derives both symbol lists with `nm -D` at test
time and asserts (a) every symbol the C `.so` defines is defined by the Rust
`.so` with the exact same name, and (b) the Rust `.so` has no undefined
symbol outside the libc/runtime allowlist above.

## Completion gate (re-checked after the final run)

| gate | status |
|------|--------|
| `nm -D`: 0 missing symbols, 0 undefined non-libc symbols in the Rust `.so` | PASS (`d1_*`, `d2_*`) |
| Phase B: every `CONFIGS.md` row (C1–C40) passes across randomized inputs | PASS (38 + 2 tests) |
| Phase C: every `ERRORS.md` row (E1–E15) has a passing differential test | PASS (15 tests) |
| Holds under EVERY feature combination (F1 = `--no-default-features`, F2 = default) and both cargo profiles | PASS (`./run_verification.sh`) |

Total: **58 differential tests**, ~7 million C-vs-Rust comparisons per run, all
performed through `dlopen`/`dlsym` on both shared objects (the Rust code is
never called as a Rust function, so the `#[unsafe(no_mangle)] extern "C"`
wrapper and the C ABI are themselves under test). No divergence was found:
`src/lib.rs` needed no fixes and is byte-identical to the input translation.
