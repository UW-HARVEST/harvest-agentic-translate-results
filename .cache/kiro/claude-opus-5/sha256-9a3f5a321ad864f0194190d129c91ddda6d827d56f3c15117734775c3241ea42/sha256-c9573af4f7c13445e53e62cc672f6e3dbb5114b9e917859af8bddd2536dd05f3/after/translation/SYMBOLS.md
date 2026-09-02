# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C `.so` exported (defined) symbols

| # | symbol | type | exported by Rust `.so`? |
|---|--------|------|-------------------------|
| 1 | `FIO_createFilename_fromOutDir` | `T` (text, global) | YES |
| 2 | `extractFilename`               | `T` (text, global) | YES |

`extractFilename` is *not* declared in `include/lib.h`, but it is a non-`static`
function in `src/lib.c`, so the C build exports it. It is therefore part of the
ABI surface and is exported (and differentially tested) from Rust as well.

## Symbol diff

```
comm -23 <c defined syms> <rust defined syms>   -> (empty)
```

**0 missing symbols.** No C source file was left untranslated: `c_src` contains
exactly one translation unit (`src/lib.c`, 53 lines) holding exactly the two
functions above.

## Undefined symbols in the Rust `.so`

All undefined/weak entries in the Rust `.so` are libc / libgcc-unwind /
Rust-runtime imports (`calloc`, `memcpy`, `strlen`, `strrchr`, `strerror`,
`fputs`, `stderr`, `exit`, `__errno_location`, `_Unwind_*`, `malloc`, `free`,
…). There are **0 missing/undefined non-libc symbols**.

## Checklist

- [x] Every C-exported symbol is exported by the Rust `.so` with the exact name.
- [x] No stubs / `unimplemented!()` — both symbols are real translations.
- [x] `nm -D` shows 0 missing or undefined non-libc symbols in the Rust `.so`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
build configuration is the default one (`cargo test`, and equivalently
`cargo test --no-default-features`). Both are exercised by
`run_all_feature_combos.sh`.
