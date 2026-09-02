# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
nm -D -u   c_src/build/libdriver.so
nm -D -u   translation/target/release/libdriver.so
```

## C source inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

| C source file | translated to | status |
|---|---|---|
| `c_src/src/driver.c` | `translation/src/lib.rs` | TRANSLATED |

`c_src/include/driver.h` declares exactly one entity: `void driver(int x);`.
There is no second module, so there is no un-translated C source. The
`SYMBOLS.md` rule "translate the missing C source" has nothing to apply to.

## Exported (defined, dynamic) symbols

| # | symbol | C `.so` | Rust `.so` | type | verdict |
|---|--------|---------|------------|------|---------|
| 1 | `driver` | `T` (0x1109) | `T` (0x116f0) | `void driver(int)` | PRESENT IN BOTH |

There are no macro-generated exports, no aliases, no versioned symbols and no
exported data objects in the C `.so`.

**Symbol diff (C exported minus Rust exported): EMPTY.**

## Imported (undefined) symbols

The C `.so` imports `printf@GLIBC_2.2.5` plus the four standard weak
ELF/CRT symbols (`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`).

The Rust `.so` imports the same `printf@GLIBC_2.2.5` — the translation
deliberately calls the *same* libc `printf` so byte output and stream
buffering are identical — plus the Rust runtime's usual libc/`libgcc`
dependencies (`malloc`, `memcpy`, `_Unwind_*`, `dl_iterate_phdr`, …).

**Non-libc / non-unwind undefined symbols in the Rust `.so`: 0.**
Every undefined symbol resolves against `libc.so.6` or `libgcc_s.so.1`,
both of which are already loaded in any process that loads the C `.so`.

## Verification checklist

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with
      the exact same name.
- [x] `nm -D` shows 0 missing / unresolvable non-libc symbols in the Rust `.so`.
- [x] No symbol is a stub, fake or `unimplemented!()`.
- [x] No C source file was left un-translated.
