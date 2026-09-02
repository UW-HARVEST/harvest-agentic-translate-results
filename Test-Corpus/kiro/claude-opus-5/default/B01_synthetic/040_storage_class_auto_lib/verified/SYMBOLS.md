# SYMBOLS.md — public symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Commands used:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
nm -D --undefined-only <each .so>
```

## Defined (exported) symbols

| # | symbol | C `.so` | Rust `.so` | type | source |
|---|--------|---------|------------|------|--------|
| 1 | `driver` | `T` (0x1109) | `T` (0x116c0) | `void driver(int x)` | `c_src/src/driver.c` |

The C library declares exactly one public entry point (`c_src/include/driver.h`
contains only `void driver(int x);`). There are no macro-generated symbols, no
exported globals/data symbols, and no additional translation units in
`CMakeLists.txt` (`add_library(driver SHARED src/driver.c)` — a single `.c`
file). Therefore no C module was left untranslated.

**Missing from Rust `.so`: none.** Symbol diff is empty.

## Undefined (imported) symbols

C `.so` imports: `printf@GLIBC_2.2.5` plus the usual weak
`_ITM_*`/`__cxa_finalize`/`__gmon_start__` stubs.

Rust `.so` imports the same `printf@GLIBC_2.2.5` (the translation calls libc
`printf` directly, so formatting and stdio buffering are shared with the C
build), plus the standard Rust runtime set: `libgcc_s` unwinder (`_Unwind_*`)
and libc/pthread routines used by `std` (`malloc`, `free`, `memcpy`, `write`,
`dl_iterate_phdr`, `pthread_key_*`, …).

**0 missing/undefined non-libc symbols:** every `U`/`w` entry in the Rust `.so`
resolves through `libc.so.6` / `libgcc_s.so.1`, both of which `ldd` reports as
found. No unresolved crate-local or project symbols remain.

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the only build
configuration is the default one. Verified with:

```sh
grep -n '\[features\]' translation/Cargo.toml   # no match
```
