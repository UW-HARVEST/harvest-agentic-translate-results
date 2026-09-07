# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Commands:

```bash
# C
cd c_src/build && nm -D --defined-only libdriver.so | sort
# Rust
cd translation && cargo build --release \
  && nm -D --defined-only target/release/libdriver.so | sort
```

## Exported (defined, global) symbols

| # | symbol | C `.so` | Rust `.so` | C source of truth |
|---|--------|---------|-----------|-------------------|
| 1 | `printIntPtrLine` | `T` | `T` | `c_src/src/driver.c:28` |
| 2 | `bad`             | `T` | `T` | `c_src/src/driver.c:33` |
| 3 | `good`            | `T` | `T` | `c_src/src/driver.c:39` |
| 4 | `driver`          | `T` | `T` | `c_src/src/driver.c:48` |

Only `driver` is declared in the public header `c_src/include/driver.h`, but the
other three have external linkage in C (no `static`), so they are part of the
`.so`'s ABI surface and MUST be exported (and are) by the Rust `cdylib`.

**Symbol diff (C-defined minus Rust-defined): EMPTY.** No stubs were used; every
symbol is a real translation of the corresponding C function.

## Undefined (imported) symbols

C `.so` imports: `printf@GLIBC_2.2.5` plus the usual weak CRT symbols
(`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

Rust `.so` imports: `printf@GLIBC_2.2.5` (the translation deliberately calls the
platform `printf` so stdout formatting/buffering is byte-identical) plus libc
allocator / unwinder / std-startup symbols (`malloc`, `memcpy`, `_Unwind_*`,
`dl_iterate_phdr`, ...). **0 missing / non-libc undefined symbols.**

## Non-symbol artifacts

There is no binary/driver executable target in `c_src/CMakeLists.txt`
(`add_library(driver SHARED ...)` only), and `translation/Cargo.toml` declares
`crate-type = ["cdylib"]` with no `[[bin]]`. Therefore the "compare binary
stdout" gate is **not applicable**; the equivalent check is performed by the
differential harness executables in `tests/` which load each `.so` and compare
captured stdout byte-for-byte.
