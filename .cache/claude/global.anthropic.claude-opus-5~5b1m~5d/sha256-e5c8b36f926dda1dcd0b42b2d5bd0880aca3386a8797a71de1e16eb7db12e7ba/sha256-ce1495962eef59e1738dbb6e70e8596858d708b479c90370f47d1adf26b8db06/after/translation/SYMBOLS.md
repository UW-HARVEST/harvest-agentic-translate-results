# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically from `nm -D` on both shared libraries.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

cd translation && cargo build --release
# -> translation/target/release/libdriver.so
```

## C `.so` exported (defined, dynamic) symbols

`nm -D --defined-only c_src/build/libdriver.so`

| # | symbol | type | source | present in Rust `.so`? |
|---|--------|------|--------|------------------------|
| 1 | `driver` | `T` (global text) | `c_src/src/driver.c:36` — `void driver(float x)` | YES (`T driver`) |

That is the complete list. The C library exports exactly one symbol.

### Non-exported C symbols (intentionally NOT in the table)

| symbol | source | why not exported |
|--------|--------|------------------|
| `print_hex` | `c_src/src/driver.c:29` | declared `static` → internal linkage, not in `nm -D`. Rust mirrors this with a private `unsafe fn print_hex`. |

`c_src/include/driver.h` declares only `void driver(float);`, so there is no
additional public API (no macros, no generated symbols, no globals, no enums, no
structs, no typedefs).

## Rust `.so` exported (defined, dynamic) symbols

`nm -D --defined-only translation/target/release/libdriver.so`

| # | symbol | type |
|---|--------|------|
| 1 | `driver` | `T` |

## Symbol diff

```
comm -3 <(nm -D --defined-only c_src/build/libdriver.so    | awk '{print $NF}' | sort -u) \
        <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort -u)
# (empty)
```

* Symbols in C but missing from Rust: **0**
* Symbols in Rust but not in C: **0**

No `#[no_mangle]` wrappers had to be added and no C source file was left
untranslated: `c_src` contains exactly one translation unit (`src/driver.c`,
40 lines incl. licence header) and one header, and both the exported `driver`
and the `static print_hex` helper are present in `translation/src/lib.rs`.

## Undefined (imported) symbols

`nm -D -u` on each library — all imports must be libc / runtime, nothing dangling.

| library | imported symbols |
|---------|------------------|
| C | `printf`, `putchar` (the compiler rewrote `printf("\n")` → `putchar('\n')`), plus the usual weak CRT symbols `_ITM_*`, `__cxa_finalize`, `__gmon_start__` |
| Rust | `printf`, `putchar` (same LLVM rewrite), plus glibc/`libgcc` runtime used by the Rust std panic/backtrace machinery: `_Unwind_*`, `__errno_location`, `abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `lseek64`, `malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`, `pthread_key_*`, `pthread_setspecific`, `read`, `readlink`, `realloc`, `realpath`, `stat64`, `strlen`, `syscall`, `write`, `writev`, weak `__cxa_thread_atexit_impl` / `gettid` / `statx` |

**0 missing / undefined non-libc symbols in the Rust `.so`.** The extra Rust
imports are all glibc or `libgcc_s` unwinder entry points pulled in by `std`;
none is an unresolved symbol from the translation itself.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the crate has
exactly one configuration: the default (empty) feature set. There are no
optional dependencies (hence no implicit features). The complete enumeration of
feature combinations to verify is therefore:

| # | cargo invocation | notes |
|---|------------------|-------|
| 1 | `cargo test --release` (default) | the only combination |
| 2 | `cargo test --release --no-default-features` | identical to #1 — no `default` feature exists |
| 3 | `cargo test --release --all-features` | identical to #1 — no features exist |

All three are run by `run_all_features.sh` and produce the same code, and the
`.so` is byte-comparable across them.
