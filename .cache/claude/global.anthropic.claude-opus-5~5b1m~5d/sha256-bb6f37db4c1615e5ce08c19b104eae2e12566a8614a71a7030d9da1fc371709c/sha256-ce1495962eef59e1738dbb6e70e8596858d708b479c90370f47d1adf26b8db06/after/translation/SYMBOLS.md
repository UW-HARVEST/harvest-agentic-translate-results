# SYMBOLS.md — public symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
nm -D --undefined-only <each>
```

## Exported (defined, dynamic) symbols

| # | symbol | in C `.so` | in Rust `.so` | notes |
|---|--------|------------|---------------|-------|
| 1 | `driver` | `T driver` | `T driver` | `void driver(int)` — the only public symbol; declared in `c_src/include/driver.h`. Exported from Rust via `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver(x: c_int)`. |

C source inventory (`grep -nE '^[a-zA-Z_].*\('  c_src/src/driver.c`) shows exactly one
function definition, `driver`, so there is no untranslated C module. There are no
macro-generated symbols, no exported data objects, no versioned symbols, and no
`static` helpers in the C translation unit.

## Symbol diff

```
C defined-only  : {driver}
Rust defined-only (filtered to C's set) : {driver}
C \ Rust        : {}      <-- EMPTY  ✅
```

**0 missing symbols.** No `#[no_mangle]` wrapper needed to be added, and no C
module was skipped by the translation.

## Undefined (imported) symbols — informational

The C `.so` imports: `printf@GLIBC_2.2.5` plus the standard weak
`_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`.

The Rust `.so` imports `printf@GLIBC_2.2.5` (the translation deliberately calls
libc `printf` rather than reimplementing formatting, so the emitted bytes and the
stdout buffering behaviour are identical) plus the usual libc/`libgcc` unwinder
symbols pulled in by the Rust standard library (`memcpy`, `malloc`, `abort`,
`_Unwind_*`, `dl_iterate_phdr`, …).

All Rust undefined symbols are libc / libgcc-unwind symbols resolved by the
dynamic loader. **0 undefined non-libc symbols.** ✅
