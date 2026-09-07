# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-FEuaO3.so

cd translation && cargo build --release
# -> translation/target/release/libhdr_compare_lib.so
```

## C `.so` exported (defined) symbols

`nm -D --defined-only c_src/build/libharvest-work-FEuaO3.so`

| # | symbol | type | source | exported by Rust `.so`? |
|---|--------|------|--------|--------------------------|
| 1 | `hdr_compare` | `T` (text, global) | `c_src/src/lib.c:9` | YES — `#[unsafe(no_mangle)] pub unsafe extern "C" fn hdr_compare` (`translation/src/lib.rs:76`) |

## C symbols deliberately NOT in the ABI

| symbol | reason |
|--------|--------|
| `hdr_valid` | declared `static` in `c_src/src/lib.c:3`; internal to the translation unit, absent from `nm -D` of the C `.so`. Reproduced in Rust as a private `unsafe fn hdr_valid` — correctly NOT exported. |

## Rust `.so` exported (defined) symbols

`nm -D --defined-only translation/target/release/libhdr_compare_lib.so`

| # | symbol | type |
|---|--------|------|
| 1 | `hdr_compare` | `T` |

## Symbol diff

```
C exports  \ Rust exports : (empty)
Rust exports \ C exports  : (empty)
```

- [x] 0 symbols missing from the Rust `.so`.
- [x] 0 extra non-libc symbols exported from the Rust `.so`.
- [x] No undefined non-libc symbols in the Rust `.so`
      (`nm -D --undefined-only` lists only libc / Rust-runtime imports; the
      crate is `panic = "abort"` and calls no external library function).

## Header surface

`c_src/include/lib.h` (3 lines) declares exactly one function:

```c
int hdr_compare(const uint8_t *h1, const uint8_t *h2);
```

There is no binary/driver target in `c_src/CMakeLists.txt` (only
`add_library(... SHARED src/lib.c)`), so the "compare binary stdout" clause of
the completion gate is not applicable.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, hence exactly one
build configuration exists (the default, which is also
`--no-default-features`). Phases B and C therefore need to be run once; both
`cargo test` and `cargo test --no-default-features` are executed to confirm.
