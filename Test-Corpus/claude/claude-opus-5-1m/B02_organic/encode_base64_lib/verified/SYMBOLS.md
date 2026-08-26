# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libdriver.so

# Rust
cargo build
nm -D --defined-only target/debug/libdriver.so
```

## C `.so` defined dynamic symbols (ground truth)

```
0000000000001163 T encode_base64
```

That is the complete list. `nm -D --defined-only` on `c_src/build/libdriver.so`
reports exactly one symbol.

## Rust `.so` defined dynamic symbols (non-mangled, non-runtime)

```
0000000000012430 T encode_base64
```

## Parity table

| # | C symbol | type | present in Rust `.so`? | Rust export site |
|---|----------|------|------------------------|------------------|
| 1 | `encode_base64` | `T` (global text) | YES — exact name | `src/lib.rs`, `#[unsafe(no_mangle)] pub unsafe extern "C" fn encode_base64` |

`comm -23 c_syms.txt r_syms.txt` (C minus Rust) → **EMPTY**. 0 missing symbols.

## Symbols intentionally NOT exported by either side

| C source symbol | reason |
|-----------------|--------|
| `static char encode(unsigned char u)` | declared `static` in `c_src/src/lib.c`, so it has internal linkage and is absent from `nm -D` of the C `.so`. The Rust translation mirrors this with a private (non-`no_mangle`) `fn encode`. Exporting it would be a *false* parity gain. |

## Undefined (imported) symbols

The C `.so` imports only libc: `calloc@GLIBC_2.2.5`, `strlen@GLIBC_2.2.5`
(plus the weak CRT/ITM stubs `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__`).

The Rust `.so` imports the same two functions it actually uses (`calloc`,
`strlen`) plus the standard Rust runtime's libc/unwind dependencies
(`_Unwind_*`, `malloc`, `free`, `memcpy`, `mmap64`, `pthread_key_create`, ...).
Every one of those resolves against `libc`/`libgcc_s` at load time.

**0 missing / 0 unresolvable non-libc undefined symbols in the Rust `.so`** —
verified by successfully `dlopen`-ing it from the differential test suite
(`libloading::Library::new` would fail on any unresolved symbol).

## Whole-module completeness check

`c_src/` contains exactly one translation unit:

```
c_src/include/lib.h   (1 line  — the single public prototype)
c_src/src/lib.c       (83 lines — encode() + encode_base64())
```

`c_src/CMakeLists.txt` compiles exactly `src/lib.c` into `libdriver.so`. Both
functions in that file are accounted for in `src/lib.rs`, so **no C module was
skipped** by the translation and no symbol required a new translation.
