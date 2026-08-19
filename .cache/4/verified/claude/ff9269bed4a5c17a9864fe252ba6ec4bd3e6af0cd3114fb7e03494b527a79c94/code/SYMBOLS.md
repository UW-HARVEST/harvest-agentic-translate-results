# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

# Rust
cargo build --no-default-features       # -> target/debug/libdriver.so
```

## 1. Translation unit inventory (completeness check)

Every C source file in `c_src` must have a Rust counterpart. There is exactly
one translation unit, so nothing can have been skipped:

| C file | lines | translated in | status |
|--------|-------|---------------|--------|
| `c_src/src/driver.c` | 37 | `src/lib.rs` | translated (both functions) |
| `c_src/include/driver.h` | 28 | `src/lib.rs` (decl of `driver`) | translated |

C functions in `driver.c`:

| C function | linkage | Rust counterpart | exported? |
|------------|---------|------------------|-----------|
| `static void print_hex(unsigned char *p, int len)` | internal (`static`) | `unsafe fn print_hex(p: *const c_uchar, len: c_int)` | no (must NOT be exported — matches C) |
| `void driver(int x)` | external | `#[unsafe(no_mangle)] pub extern "C" fn driver(x: c_int)` | yes |

No C source file is missing from the Rust translation; no symbol is stubbed or
`unimplemented!()`.

## 2. `nm -D --defined-only` — defined (exported) dynamic symbols

### C `libdriver.so`

```
0000000000001173 T driver
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
```

### Rust `libdriver.so`

```
00000000000121f0 T driver
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __cxa_thread_atexit_impl@GLIBC_2.18
                 w __gmon_start__
                 w gettid@GLIBC_2.30
```

(The `w` entries are toolchain/CRT weak references, not library API.)

## 3. Symbol parity table

| # | C symbol (`nm -D`) | type | exported by Rust `.so`? | notes |
|---|--------------------|------|-------------------------|-------|
| 1 | `driver` | `T` (global text) | **YES** — `T driver` | the only public API symbol |

### Symbols intentionally NOT exported by either side

| C symbol | why | C `.so` exports it? | Rust `.so` exports it? |
|----------|-----|---------------------|------------------------|
| `print_hex` | `static` in C → internal linkage | no | no (private `unsafe fn`) |

### Diff result

```sh
comm -23 <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $NF}' | sort) \
         <(nm -D --defined-only target/debug/libdriver.so | awk '{print $NF}' | sort)
# (empty)
```

**Missing symbols: 0.** The symbol diff is EMPTY.

## 4. Undefined (imported) symbols in the Rust `.so`

All undefined symbols resolve to glibc / libgcc-unwind / weak CRT hooks — there
are **0 missing non-libc symbols**:

* glibc: `printf`, `memcpy`, `memmove`, `memset`, `bcmp`, `strlen`, `malloc`,
  `calloc`, `realloc`, `free`, `posix_memalign`, `abort`, `write`, `writev`,
  `read`, `close`, `open64`, `lseek64`, `stat64`, `fstat64`, `statx`,
  `readlink`, `realpath`, `getcwd`, `getenv`, `mmap64`, `munmap`, `syscall`,
  `dl_iterate_phdr`, `__errno_location`, `__tls_get_addr`, `pthread_key_create`,
  `pthread_key_delete`, `pthread_setspecific`
* libgcc unwinder (Rust panic machinery): `_Unwind_*`
* weak CRT/toolchain hooks: `_ITM_*`, `__cxa_finalize`,
  `__cxa_thread_atexit_impl`, `__gmon_start__`, `gettid`

The C `.so` imports `printf` and `putchar` (GCC rewrites the argument-less
`printf("\n")` into `putchar('\n')`). The Rust side calls `printf("\n")`
directly. This is an *implementation* difference only — the emitted byte stream
on `stdout` is identical, which Phase B verifies differentially.

The extra glibc/unwind imports on the Rust side come from the Rust standard
library (present in every `cdylib`); they add no public API surface, so symbol
parity is unaffected.

## 5. Feature combinations

`Cargo.toml` has **no `[features]` table**, and `c_src/CMakeLists.txt` defines
**no compile options / `option()` / `target_compile_definitions`**. Therefore
the complete set of build configurations is a single one:

| # | cargo invocation | cmake configuration |
|---|------------------|---------------------|
| 1 | `--no-default-features` (empty feature set) | default (`add_library(driver SHARED src/driver.c)`) |

`cargo check --no-default-features` → clean (0 errors, 0 warnings).

## 6. Phase D result (automated)

`tests/symbols.rs` re-derives this table at test time (`nm -D` on both `.so`s):

```
test phase_d_every_c_symbol_is_exported_by_rust ... ok
test phase_d_internal_c_symbols_stay_internal ... ok
test phase_d_rust_has_no_unresolved_non_libc_symbols ... ok
```

`./run_tests.sh` performs the same diff for every feature combination:

```
--- nm -D symbol diff ---
symbol diff empty (0 missing)
```

Verified for both the `dev` and the `release` profile (`release` additionally
carries `panic = "abort"`); the symbol diff is empty in both cases.
