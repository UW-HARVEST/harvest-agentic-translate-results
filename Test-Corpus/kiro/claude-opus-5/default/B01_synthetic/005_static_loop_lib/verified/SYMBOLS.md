# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libStaticLoop.so
nm -D --defined-only translation/target/release/libStaticLoop.so
```

## C `.so` public (defined, dynamic) symbols

| # | symbol | C type | present in Rust `.so` | notes |
|---|--------|--------|-----------------------|-------|
| 1 | `driver`     | `T` (global text) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver(c_int)` |
| 2 | `static_sum` | `T` (global text) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn static_sum(c_int) -> c_int` |

Total C exported symbols: **2**. Total also exported by Rust: **2**.
Symbol diff (C exports missing from Rust): **EMPTY**.

There are no macro-generated symbols in this library (the C source contains no
symbol-defining macros), and no exported C data objects — the only mutable
state, `static int sum` inside `static_sum`, has *no linkage* in C (it is a
function-local static) and is correspondingly a private `static mut SUM` in
Rust. Neither `.so` exports it, so parity holds.

## Undefined (imported) symbols

Both libraries import only libc / toolchain runtime symbols; there are no
missing non-libc symbols in the Rust `.so`.

* C imports: `printf@GLIBC_2.2.5`, plus the standard weak
  `_ITM_*` / `__cxa_finalize` / `__gmon_start__` glibc-init set.
* Rust imports: `printf@GLIBC_2.2.5` (the translation calls libc `printf`
  directly so stdout formatting *and* buffering match), plus the Rust `std`
  runtime's usual libc/`_Unwind_*` set (`malloc`, `memcpy`, `write`, `writev`,
  `dl_iterate_phdr`, ...). Every one resolves from `libc`/`libgcc_s`.

**0 missing/undefined non-libc symbols in the Rust `.so`.** ✅

## Module-completeness check

`c_src` contains exactly one translation unit (`src/staticloop.c`, 43 lines)
and one public header (`include/staticloop.h`). Both public declarations in the
header (`static_sum`, `driver`) are translated in `translation/src/lib.rs`.
No C module was skipped; nothing is stubbed or `unimplemented!()`.
