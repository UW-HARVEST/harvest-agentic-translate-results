# SYMBOLS.md — public symbol surface

Derived mechanically from `nm -D` on both shared objects.

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source surface

`c_src/` contains exactly one translation unit (`src/lib.c`, 90 lines) and one
public header (`include/lib.h`, 1 line):

```c
char *searchAndReplace(const char *orig, const char *search, const char *value);
```

There are no macros that generate symbol names, no `#ifdef`-gated extra
entry points, no additional `.c` files, and no binary/driver target in
`CMakeLists.txt` (`add_library(driver SHARED src/lib.c)` only). So the entire
public surface is a single function; no C module was left untranslated.

## Defined dynamic symbols

| # | symbol | in C `.so` | in Rust `.so` | notes |
|---|--------|-----------|---------------|-------|
| 1 | `searchAndReplace` | yes (`T`) | yes (`T`) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` in `src/lib.rs` |

Symbol diff (C defined ∖ Rust defined): **empty**.
Symbol diff (Rust defined ∖ C defined): **empty**.

## Undefined (imported) symbols

Both objects import only libc / toolchain symbols; none are unresolved
project symbols.

* C imports: `malloc`, `realloc`, `strdup`, `strlen`, `strncpy`, `strstr`,
  `__cxa_finalize`, `__gmon_start__`, `_ITM_*`.
* Rust imports: the same `malloc`, `realloc`, `strdup`, `strlen` plus the usual
  Rust-runtime libc/unwind set (`memcpy`, `memmove`, `memset`, `free`,
  `_Unwind_*`, `dl_iterate_phdr`, …). All are provided by `libc.so.6` /
  `libgcc_s.so.1`; `ldd` resolves every one.

`strncpy` and `strstr` are absent from the Rust imports because the
translation open-codes them (`c_strncpy`, `c_strstr`) — this is an
implementation detail, not a missing export.

## Result

- [x] 0 symbols missing from the Rust `.so`.
- [x] 0 extra non-libc symbols exported by the Rust `.so`.
- [x] 0 unresolved non-libc undefined symbols in the Rust `.so`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
feature configuration that exists is the default (empty) one. `cargo check
--no-default-features` is equivalent to the default build. Verified by
`grep -n '^\[features\]' Cargo.toml` → no match.
