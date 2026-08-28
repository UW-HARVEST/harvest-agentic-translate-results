# SYMBOLS.md — dynamic-symbol parity between the C `.so` and the Rust `.so`

Generated mechanically. Reproduce with:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libharvest-work-QUHmNR.so | sort
# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libcolourblind_lib.so | sort
```

`scripts/symbol_parity.sh` in the crate root automates the whole diff and exits
non-zero if anything the C exports is missing from the Rust.

## Translation-unit inventory (completeness check)

The C library is a single translation unit. Every function in it is accounted
for below, so no module was skipped by the translation.

| C source file | function | C linkage | translated as | exported? |
|---|---|---|---|---|
| `c_src/src/lib.c:3`  | `Protanopia`   | `static` (internal) | `protanopia` (private `unsafe fn`)   | no — `static` in C, not in `nm -D` |
| `c_src/src/lib.c:10` | `Deuteranopia` | `static` (internal) | `deuteranopia` (private `unsafe fn`) | no — `static` in C, not in `nm -D` |
| `c_src/src/lib.c:17` | `Tritanopia`   | `static` (internal) | `tritanopia` (private `unsafe fn`)   | no — `static` in C, not in `nm -D` |
| `c_src/src/lib.c:24` | `colourblind`  | external            | `#[no_mangle] pub unsafe extern "C" fn colourblind` | **yes** |

`c_src/include/lib.h` declares one type (`enum cb_impairment`, a compile-time
construct with no symbol) and one function (`colourblind`). There are no
macro-generated symbols, no global/`extern` variables, no constructors
(`__attribute__((constructor))`), and no `#ifdef`-gated alternates in the C, so
the exported surface cannot vary by build configuration.

## `nm -D --defined-only` — C `.so`

```
00000000000013d2 T colourblind
```

Count: **1** defined dynamic symbol.

## `nm -D --defined-only` — Rust `.so`

```
0000000000011c70 T colourblind
```

Count: **1** defined dynamic symbol.

## Diff

| symbol | in C `.so` | in Rust `.so` | verdict |
|---|---|---|---|
| `colourblind` | T | T | ✅ present in both, exact name match |

**Symbols exported by C but missing from Rust: 0.**
**Symbols exported by Rust but not by C: 0** (no accidental surface widening —
the crate is a `cdylib`, and the three helpers are private, mirroring `static`).

## Undefined / imported symbols

Neither `.so` imports a non-libc symbol.

C `.so` undefined entries (all weak or libc, all expected CRT glue):

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
```

The Rust `.so` needs no libc symbol at all for `colourblind`: the translation is
pure arithmetic with no allocation, no I/O and no panic machinery on the hot
path (`panic = "abort"` is set for `release`).

**0 missing / undefined non-libc symbols in the Rust `.so`.** ✅
