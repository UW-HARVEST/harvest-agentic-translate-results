# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libdriver.so

# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory (completeness check)

The whole library is a single translation unit, so there is no possibility of a
"skipped module":

| C file | translated? | notes |
|--------|-------------|-------|
| `c_src/src/lib.c` | yes | both functions present in `translation/src/lib.rs` |
| `c_src/include/lib.h` | yes | declares the single public symbol `encode_base64` |

`c_src/CMakeLists.txt` builds exactly one target: `add_library(driver SHARED src/lib.c)`.
There is no conditional compilation (`grep '#if\|#ifdef\|#ifndef'` over the C
sources returns nothing), so there is only ONE C build configuration.

## Defined (exported) symbols

`nm -D --defined-only` on the C `.so` yields exactly one symbol:

| # | C symbol | type | exported by Rust `.so`? | Rust item |
|---|----------|------|--------------------------|-----------|
| 1 | `encode_base64` | `T` (global text) | YES — `T encode_base64` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn encode_base64` |

### Non-exported C functions (must NOT appear in either `.so`)

| C symbol | linkage | in C `.so` dyn table? | in Rust `.so` dyn table? |
|----------|---------|------------------------|---------------------------|
| `encode` | `static char encode(unsigned char)` | no (internal) | no (private `fn encode`) |

## Symbol diff

```
symbols in C .so but missing from Rust .so:   (none)
```

The diff is EMPTY. No `#[no_mangle]` wrapper had to be added and no C module
was left untranslated, so no stubbing was required or performed.

## Undefined (imported) symbols

The C `.so` imports `calloc@GLIBC` and `strlen@GLIBC`. The Rust `.so` imports
both of these as well (it calls the real libc `calloc`/`strlen` through
`unsafe extern "C"` declarations, so the returned buffer is `free()`-able by the
caller exactly as in C). The Rust `.so` additionally imports the usual Rust
runtime/libc support symbols (`_Unwind_*`, `memcpy`, `malloc`, `abort`, …); all
of these resolve from `libc`/`libgcc_s` and none is an unresolved non-libc
symbol.

```
non-libc / non-runtime undefined symbols in Rust .so: 0
```

## Completion gate item

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
