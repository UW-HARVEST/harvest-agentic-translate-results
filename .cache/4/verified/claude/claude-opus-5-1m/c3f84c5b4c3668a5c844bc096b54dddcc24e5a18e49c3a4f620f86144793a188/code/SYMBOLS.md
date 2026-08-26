# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically from `nm -D --defined-only` on both shared objects.

## Build configuration surface

| source | configuration knobs | conclusion |
|--------|--------------------|------------|
| `Cargo.toml` | **no `[features]` section at all** | exactly one Rust build config (default == `--no-default-features`) |
| `c_src/CMakeLists.txt` | no `option()`, no `if()`, no `add_definitions`, single source `src/lib.c` | exactly one C build config |
| `c_src/src/lib.c`, `c_src/include/lib.h` | no `#if` / `#ifdef` / `#ifndef` anywhere | no preprocessor variants |

=> **Total valid feature combinations: 1** (the empty set). Verified with
`cargo check --no-default-features` and `cargo check` — both clean, 0 warnings/errors.
`tests/phase_d_symbol_parity.rs::d4_exactly_one_feature_combination` re-derives
this from `Cargo.toml`/`CMakeLists.txt`/`lib.c` on every run, so if a feature or
`#ifdef` is ever added the suite fails instead of silently under-testing.

## One Cargo.toml change was required for the tests to be meaningful

`crate-type` was `["cdylib"]` and is now `["lib", "cdylib"]`.

With `cdylib` alone, nothing under `tests/` depends on the lib target, so
**`cargo test` never rebuilt `libcleanup_lib.so`** — the suite silently loaded a
stale artifact and every injected bug went undetected (confirmed: 14/14 mutations
survived). Adding `"lib"` makes the integration tests depend on the lib target,
so cargo rebuilds the cdylib before running them. The tests still load the
cdylib exclusively through `libloading`/`dlsym`; they never link or call the rlib.
`tests/common/mod.rs::assert_so_is_fresh` additionally fails the run if the `.so`
is older than any file in `src/`, so this cannot silently regress.

Note that `cargo test` refreshes `target/debug/deps/libcleanup_lib.so` but not
`target/debug/libcleanup_lib.so`; the harness therefore prefers the `deps/` copy.

## Commands used

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so

# Rust
cargo build --no-default-features
# -> target/debug/libcleanup_lib.so

nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/debug/libcleanup_lib.so
```

## Exported (defined) dynamic symbols

| # | symbol | C `.so` | Rust `.so` | C signature | Rust export |
|---|--------|---------|------------|-------------|-------------|
| 1 | `cleanup` | `T` | `T` | `int cleanup(int a, int b, int c, int d)` | `#[unsafe(no_mangle)] pub extern "C" fn cleanup` |
| 2 | `print_result` | `T` | `T` | `void print_result(const char *label, int result)` | `#[unsafe(no_mangle)] pub extern "C" fn print_result` |
| 3 | `cleanup_resources` | `T` | `T` | `void cleanup_resources(char *dynamic_str)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn cleanup_resources` |

Note: only `cleanup` is declared in the public header `include/lib.h`.
`print_result` and `cleanup_resources` are declared/defined in `src/lib.c` with
external linkage, so they are part of the exported ABI surface and are tested as
public entry points.

## Symbol diff

```
comm -23 c_syms.txt rust_syms.txt   # C has, Rust lacks
<empty>
comm -13 c_syms.txt rust_syms.txt   # Rust has, C lacks
<empty>
```

**Missing symbols: 0. Extra symbols: 0. Symbol diff is EMPTY.**

No symbol required translating a previously-skipped C module: `src/lib.c` is the
only C translation unit and all three of its external-linkage functions are
implemented in `src/lib.rs` (no stubs, no `unimplemented!()`).

## Undefined (imported) symbols in the Rust `.so`

All imports are libc / libgcc-unwind / Rust-runtime only — **0 missing
non-libc symbols**:

`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `_Unwind_*@GCC_*`,
`__cxa_finalize`, `__cxa_thread_atexit_impl`, `__errno_location`,
`__gmon_start__`, `__tls_get_addr`, `abort`, `bcmp`, `calloc`, `close`,
`dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `gettid`, `lseek64`,
`malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`,
`posix_memalign`, `printf`, `pthread_key_create`, `pthread_key_delete`,
`pthread_setspecific`, `read`, `readlink`, `realloc`, `realpath`, `snprintf`,
`stat64`, `statx`, `strlen`, `strncmp`, `syscall`, `write`, `writev`

The four libc functions the translation deliberately shares with the C build —
`printf`, `snprintf`, `strlen`, `strncmp`, plus `malloc`/`free` — are imported
from the same glibc the C `.so` uses, which is what makes stdout formatting and
buffering byte-identical.
