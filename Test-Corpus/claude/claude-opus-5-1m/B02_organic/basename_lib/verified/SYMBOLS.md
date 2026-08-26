# SYMBOLS.md — Public ABI surface (Phase A)

Derived mechanically from `nm -D` on the built shared libraries.

Build commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

# Rust (only one configuration exists, see CONFIGS.md)
cargo build --offline --release --no-default-features   # -> target/release/libdriver.so
cargo build --offline --no-default-features             # -> target/debug/libdriver.so
```

## C library exported (defined) dynamic symbols

`nm -D --defined-only c_src/build/libdriver.so`

| # | symbol | type | defined in C source | exported by Rust `.so`? |
|---|--------|------|---------------------|-------------------------|
| 1 | `tool_basename` | `T` (global text) | `c_src/src/lib.c:5` | YES — `src/lib.rs`, `#[unsafe(no_mangle)] pub unsafe extern "C" fn tool_basename` |

The C library declares exactly one public entry point; `c_src/include/lib.h`
contains exactly one line:

```c
char *tool_basename(char *path);
```

`find c_src -name '*.c' -o -name '*.h'` shows only `c_src/src/lib.c` and
`c_src/include/lib.h` (plus CMake's own generated compiler-probe file, which is
not part of the library). There is therefore **no untranslated C module**: the
whole library is one function, and it is translated.

## Weak / undefined symbols in the C `.so` (not part of the ABI)

| symbol | kind | note |
|--------|------|------|
| `_ITM_deregisterTMCloneTable` | `w` | toolchain-generated weak stub |
| `_ITM_registerTMCloneTable` | `w` | toolchain-generated weak stub |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | glibc |
| `__gmon_start__` | `w` | toolchain-generated weak stub |
| `strrchr@GLIBC_2.2.5` | `U` | libc import, used by `tool_basename` |

## Symbol diff (Phase D gate)

```sh
nm -D --defined-only c_src/build/libdriver.so   | awk '{print $3}' | sort > c_syms.txt
nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort > rust_syms.txt
comm -23 c_syms.txt rust_syms.txt    # symbols in C but missing from Rust
```

Result: **empty** — 0 symbols missing from the Rust `.so` (verified for both
`target/debug/libdriver.so` and `target/release/libdriver.so`).

The Rust `.so` exports exactly one non-`std` symbol, `tool_basename`, i.e. it
does not export anything the C library does not.

## Undefined symbols in the Rust `.so`

All `U` entries in the Rust `.so` are libc / libgcc-unwind imports pulled in by
the Rust standard library (`malloc`, `memcpy`, `strlen`, `_Unwind_*`,
`pthread_key_*`, `dl_iterate_phdr`, …). There are **0 undefined non-libc
symbols**, i.e. no unresolved references to code that was never translated.

Verification script: `check_symbols.sh` (run it to reproduce the diff).
