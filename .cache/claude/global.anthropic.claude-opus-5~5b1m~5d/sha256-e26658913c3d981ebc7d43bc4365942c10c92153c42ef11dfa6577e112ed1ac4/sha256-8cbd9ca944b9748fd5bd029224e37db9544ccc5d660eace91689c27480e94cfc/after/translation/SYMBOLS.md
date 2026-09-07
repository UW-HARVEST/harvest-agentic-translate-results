# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

- C  `.so`: `c_src/build/libdriver.so`
- Rust `.so`: `translation/target/release/libdriver.so`

## Exported (defined, dynamic) symbols

| # | symbol | C `.so` | Rust `.so` | C declaration | notes |
|---|--------|---------|------------|---------------|-------|
| 1 | `printLine` | T | T | `void printLine(const char *line)` | only symbol taking a parameter; NULL-guarded |
| 2 | `bad`       | T | T | `void bad(void)`    | prints `bad()` |
| 3 | `good`      | T | T | `void good(void)`   | prints `good()` then calls `static helperGood` |
| 4 | `driver`    | T | T | `void driver(void)` | public entry point from `include/driver.h` |

`nm -D --defined-only` output, verbatim:

```
C:                          Rust:
T bad                       T bad
T driver                    T driver
T good                      T good
T printLine                 T printLine
```

**Symbol diff (C-exported minus Rust-exported): EMPTY.** ✔

## Deliberately NOT exported (matches C internal linkage)

| symbol | C linkage | Rust |
|--------|-----------|------|
| `helperBad`  | `static void helperBad()`  — internal, never called by the C TU | private `unsafe fn`, kept alive via `#[used]` static, not in dynsym ✔ |
| `helperGood` | `static void helperGood()` — internal, called from `good()` | private `unsafe fn`, not in dynsym ✔ |

Verified: `nm -D translation/target/release/libdriver.so | grep -E 'helperBad|helperGood'` → no output,
same as for the C `.so`.

## Undefined (imported) symbols

The C `.so` imports exactly one non-weak libc symbol: `puts@GLIBC_2.2.5`
(the C compiler lowers `printf("%s\n", line)` to `puts(line)`).

The Rust `.so` also imports `puts@GLIBC_2.2.5` — the translation calls libc
`puts` directly rather than `std::io`, so both libraries share the *same*
`stdout` `FILE` stream and the same buffering discipline. Every other Rust
undefined symbol is libc (`malloc`, `memcpy`, `write`, `open64`, …) or the
`_Unwind_*` / `__cxa_*` personality-and-TLS machinery pulled in by `libstd`.

**Non-libc, non-runtime undefined symbols in Rust: 0.** ✔

## Completeness

`c_src` contains exactly one translation unit (`src/driver.c`, 66 lines) and one
header (`include/driver.h`). No C source file was skipped; no symbol required a
new translation or a stub.
