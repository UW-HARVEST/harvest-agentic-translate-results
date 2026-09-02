# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-5xJDIq.so

cd translation && cargo build --release
# -> translation/target/release/librgb_to_hsv_lib.so
```

## C `.so` — defined dynamic symbols (`nm -D --defined-only`)

| # | symbol | type | present in Rust `.so`? | notes |
|---|--------|------|------------------------|-------|
| 1 | `rgb_to_hsv` | `T` (global text) | YES (`T rgb_to_hsv`) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn rgb_to_hsv` in `src/lib.rs` |

That is the complete list. `c_src/CMakeLists.txt` compiles exactly one
translation unit (`src/lib.c`), and `c_src/include/lib.h` declares exactly one
function, so there is no untranslated C module. Nothing is stubbed.

## Symbol diff

```
comm -23 <(nm -D --defined-only C.so   | awk '{print $NF}' | sort -u) \
         <(nm -D --defined-only RUST.so| awk '{print $NF}' | sort -u)
```

Result: **EMPTY** — 0 symbols exported by the C `.so` are missing from the Rust `.so`.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind
imports pulled in by the Rust runtime (`malloc`, `memcpy`, `_Unwind_*`,
`pthread_key_*`, `dl_iterate_phdr`, `abort`, …). There are **0 undefined
non-libc symbols** — i.e. no dangling references to untranslated code.

Extra symbols the Rust `.so` exports beyond the C set are Rust-internal
(`_ZN…` / `_R…` mangled std items and `__rust_*` allocator shims). Extra
symbols are permitted; the gate is that no C symbol is missing.

## Gate status

- [x] `nm -D` shows 0 missing symbols in the Rust `.so` relative to the C `.so`.
- [x] `nm -D` shows 0 undefined non-libc symbols in the Rust `.so`.
