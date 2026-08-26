# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## What is being compared

`c_src/CMakeLists.txt` declares a single target:

```cmake
add_executable(driver src/main.c)
```

so the *primary* artifact of the C project is an **executable**, not a library.
A PIE executable exports no dynamic symbols at all
(`nm -D --defined-only c_src/build/driver` → empty), which would make symbol
parity vacuous. To get a non-vacuous, FFI-level comparison the identical
`c_src/src/main.c` translation unit is therefore *also* built as a shared
library (same compiler, same default = unoptimised flags, nothing in `c_src/`
modified):

```sh
gcc -shared -fPIC -o target/csrc/libcdriver.so c_src/src/main.c
```

and the Rust crate grew a `crate-type = ["cdylib"]` `[lib]` target
(`src/lib.rs`) whose `#[no_mangle] extern "C"` wrappers mirror that surface
(`target/debug/libdriver.so`). Both `.so`s are what `tests/` dlopen()s through
`libloading`; the Rust side is *never* called directly as a Rust function.

Regenerate and re-diff with:

```sh
./build_all.sh          # builds C exe, C .so, Rust exe, Rust .so, checks parity
```

## `nm -D --defined-only` on the C `.so` → every symbol, and its Rust status

| # | C symbol | C type | Rust `.so` exports it | Rust implementation |
|---|----------|--------|-----------------------|---------------------|
| 1 | `printLine` | `T` (FUNC GLOBAL) | ✅ yes | `src/lib.rs::printLine` → `prog::print_line` |
| 2 | `bad`       | `T` (FUNC GLOBAL) | ✅ yes | `src/lib.rs::bad` → `prog::bad` |
| 3 | `good`      | `T` (FUNC GLOBAL) | ✅ yes | `src/lib.rs::good` → `prog::good` |
| 4 | `main`      | `T` (FUNC GLOBAL) | ✅ yes | `src/lib.rs::main` → `prog::run` |

`comm -23 c.syms r.syms` (symbols in C but not in Rust) is **empty — 0
missing**. No symbol is stubbed: every one runs the real translated logic.
There are no macro-generated symbols in this source, and no data symbols
(`B`/`D`/`R`) — the four functions above are the complete C surface.

Signatures used by the wrappers (from `c_src/src/main.c`):

| C declaration | Rust wrapper |
|---|---|
| `void printLine(const char *line)` | `unsafe extern "C" fn printLine(line: *const c_char)` |
| `void bad(void)` | `unsafe extern "C" fn bad()` |
| `void good(void)` | `unsafe extern "C" fn good()` |
| `int main(void)` | `unsafe extern "C" fn main() -> c_int` |

There are no headers in `c_src/` (single translation unit), so the signatures
are taken from the definitions themselves. `main` is exported by the Rust
cdylib under `#[cfg(not(test))]` only, so it cannot collide with the `main`
libtest generates when `src/lib.rs` is compiled as a unit-test harness.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only target/debug/libdriver.so` lists **only** libc / libgcc
/ ld.so imports — `__errno_location`, `read`, `write`, `close`, `malloc`,
`calloc`, `realloc`, `free`, `posix_memalign`, `memcpy`, `memmove`, `memset`,
`bcmp`, `strlen`, `abort`, `getenv`, `getcwd`, `readlink`, `realpath`,
`open64`, `lseek64`, `fstat64`, `stat64`, `statx`, `mmap64`, `munmap`,
`writev`, `syscall`, `gettid`, `dl_iterate_phdr`, `pthread_key_create`,
`pthread_key_delete`, `pthread_setspecific`, `__cxa_thread_atexit_impl`,
`__cxa_finalize`, `__tls_get_addr`, the `_Unwind_*` family and the weak
`_ITM_*` / `__gmon_start__` optimisation hooks.

**0 non-libc undefined symbols.** `ldd target/debug/libdriver.so` resolves
everything (`libgcc_s.so.1`, `libc.so.6`, `ld-linux-x86-64.so.2`) with no
"not found" entries.

The C `.so` for comparison imports `__isoc99_scanf` and `puts` (gcc lowers
`printf("%s\n", line)` to `puts(line)` — same output bytes); the Rust side
reaches the same syscalls through `read`/`write` instead, which is not
observable in the produced byte stream.

## Tests enforcing this file

`tests/symbols.rs`:

| test | what it enforces |
|---|---|
| `symbol_parity_c_so_subset_of_rust_so` | `nm -D` diff (C minus Rust) is empty, and the C surface is still exactly those 4 names |
| `every_c_symbol_is_resolvable_in_both_libraries` | each name resolves through `dlsym` in *both* `.so`s — present in the table is not enough |
| `rust_so_has_no_unresolved_non_libc_symbols` | `ldd` reports no "not found", and the libc imports actually used are present |
| `smoke_executables_run_and_agree` | both executables run and agree |
| `smoke_shared_libraries_load_and_agree` | both `.so`s load via `libloading` and agree |
| `smoke_so_main_agrees` | the exported `main` agrees through both `.so`s |

`build_all.sh` re-runs the same `nm -D` diff and the `ldd` check on every build,
so a regression fails the build and not just the test-suite.

## Result

- [x] Every symbol the C `.so` exports is exported by the Rust `.so` with the
      exact same name (4/4).
- [x] 0 missing symbols, 0 stubs, 0 `unimplemented!()`.
- [x] 0 unresolved non-libc undefined symbols in the Rust `.so`.
- [x] Holds for both cargo profiles (`dev` and `release`) and all three feature
      invocations — see CONFIGS.md and `./verify.sh`.
