# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

cd translation && cargo build --release
# -> translation/target/release/libdriver.so
```

## C `.so` exported (defined) dynamic symbols

`nm -D --defined-only c_src/build/libdriver.so`

| # | symbol | type | source | exported by Rust `.so`? |
|---|--------|------|--------|-------------------------|
| 1 | `driver` | `T` (global text) | `c_src/src/driver.c:36`, declared `c_src/include/driver.h:27` | YES — `T driver` |

Total C defined dynamic symbols: **1**. Total missing from Rust `.so`: **0**.

## Not exported, by design

| C symbol | why absent from `nm -D` in BOTH `.so`s |
|----------|----------------------------------------|
| `print_hex` | declared `static void print_hex(unsigned char *p, int len)` in `c_src/src/driver.c:29` → internal linkage, absent from the C `.so` dynamic table. The Rust translation mirrors this as a private `unsafe fn print_hex` (no `#[no_mangle]`), so it is likewise absent. Adding it would be a *parity violation*, not a fix. |

No macro-generated symbols exist: `c_src/src/driver.c` contains no function-defining
macros (`grep -nE '#if|#ifdef|#ifndef' c_src/src/` → no matches).

## Undefined symbols

The C `.so` imports `printf@GLIBC_2.2.5` and `putchar@GLIBC_2.2.5` (GCC lowers
the source-level `printf("\n")` at `driver.c:32` into a `putchar` call), plus the
usual weak CRT hooks (`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

The Rust `.so` imports the same `printf` and `putchar` — the translation binds
libc's stdio deliberately rather than using `std::io::stdout`, so both libraries
write through the *same* `FILE *stdout` with identical buffering semantics. Its
remaining undefined symbols are all libc / libgcc-unwind / Rust-std runtime
(`malloc`, `memcpy`, `_Unwind_*`, `pthread_key_*`, `dl_iterate_phdr`, …).

**0 missing or undefined non-libc symbols in the Rust `.so`.**

## Feature combinations

`translation/Cargo.toml` has no `[features]` section and no `optional`
dependencies, so exactly **one** feature combination exists (the empty/default
set). `--no-default-features` is therefore identical to the default build; both
are exercised in Phase D.

## Binary / driver executable

`c_src/CMakeLists.txt` contains no `add_executable`, and `translation/Cargo.toml`
declares `crate-type = ["cdylib"]` with no `[[bin]]`. Neither project builds an
executable, so the "compare C and Rust binary stdout" gate is **not applicable**;
stdout is instead compared through the FFI boundary by fd-redirect capture in the
differential tests.
