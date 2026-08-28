# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D` on the built C shared library:

```
c_src/build/libharvest-work-PzcuMI.so
translation/target/release/libbin2hex_lib.so
```

## C source inventory (completeness check)

The whole C subtree is:

```
c_src/CMakeLists.txt
c_src/include/lib.h      # 1 declaration
c_src/src/lib.c          # 1 definition
```

`CMakeLists.txt` compiles exactly one translation unit (`src/lib.c`) into
`SHARED` library `${project_name}`. There are no other `.c` files, no
`#ifdef`-guarded alternate implementations, no namespacing/renaming macros
(e.g. no `#define bin2hex sodium_bin2hex`), and no macro-generated symbol
families. Therefore the exported surface is exactly one function and **no C
module was skipped by the translation**.

`grep -c 'return\|abort' c_src/src/lib.c` → the function has a single `return`
and a single `abort()`; see `ERRORS.md`.

## Defined (exported) symbols

`nm -D --defined-only` output, filtered of linker-synthesised entries
(`_ITM_*`, `__cxa_finalize`, `__gmon_start__`, `__cxa_thread_atexit_impl`,
`gettid`, `statx` — all weak/undefined placeholders, not API):

| # | C symbol (`nm -D`) | type | Rust `.so` exports it? | how |
|---|--------------------|------|------------------------|-----|
| 1 | `bin2hex`          | `T`  | YES — `T bin2hex`      | `#[unsafe(no_mangle)] pub unsafe extern "C" fn bin2hex` in `src/lib.rs` |

**Missing from Rust `.so`: none.** No stubs, no `unimplemented!()`; the single
symbol is a real translation of `c_src/src/lib.c`.

## Signature

```c
char *bin2hex(char *hex, size_t hex_maxlen, const uint8_t *bin, size_t bin_len);
```

```rust
pub unsafe extern "C" fn bin2hex(
    hex: *mut c_char, hex_maxlen: usize, bin: *const u8, bin_len: usize,
) -> *mut c_char
```

ABI check: `char*` ↔ `*mut c_char`, `size_t` ↔ `usize`, `const uint8_t*` ↔
`*const u8`, return `char*` ↔ `*mut c_char`. All four arguments are register
sized, no aggregates, so the SysV x86-64 ABI mapping is exact.

## Undefined (imported) symbols

The C `.so` imports exactly one non-weak external: `abort@GLIBC_2.2.5`.

The Rust `.so` imports `abort@GLIBC_2.2.5` too (it calls real libc `abort()`,
not a Rust panic, so the process dies from `SIGABRT` identically). Its other
undefined symbols (`malloc`, `memcpy`, `_Unwind_*`, `dl_iterate_phdr`, …) are
libc / libgcc / Rust-std runtime imports, **not** unresolved library symbols.

`nm -D` non-libc undefined symbols in the Rust `.so`: **0**.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section** and no optional
dependencies, so the complete feature power set is a single element: the
default (empty) feature set. `cargo tree -e features` and the automated sweep
in `tests/feature_matrix.sh` confirm there is exactly one combination to
verify, and Phases B–C are run under it.

## Verdict

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`.
- [x] `nm -D` shows 0 missing/undefined non-libc symbols in the Rust `.so`.
