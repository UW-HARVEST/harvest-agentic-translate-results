# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D --defined-only` on the C build of `libdriver.so`
(`c_src/build/libdriver.so`), compared against the Rust `cdylib`
(`translation/target/release/libdriver.so`).

The whole C library is a single translation unit, `c_src/src/driver.c` (83 lines,
5 functions). There is no second module, so there is no un-translated C source.

## Exported (dynamic, defined) symbols

| # | symbol | C decl | in C `.so` | in Rust `.so` | notes |
|---|--------|--------|-----------|---------------|-------|
| 1 | `printLine`    | `void printLine(const char *line)` | T | T | not in `driver.h`; still an exported public symbol |
| 2 | `printIntLine` | `void printIntLine(int intNumber)`  | T | T | not in `driver.h`; still exported |
| 3 | `bad`          | `void bad(void)`                    | T | T | not in `driver.h`; still exported |
| 4 | `good`         | `void good(void)`                   | T | T | not in `driver.h`; still exported |
| 5 | `driver`       | `void driver(int useGood)`           | T | T | the only symbol declared in `include/driver.h` |

No macro-generated symbols exist in this library (`grep` finds no function-like
macros in `driver.c` / `driver.h`).

## Symbol diff

```
comm -23 <c defined syms> <rust defined syms>   ->  (empty)
```

0 symbols missing from the Rust `.so`. The Rust `.so` defines exactly these five
and no extra public C symbols.

## Undefined (imported) symbols

C `.so` imports: `printf`, `puts` (gcc rewrites `printf("%s\n", s)` into
`puts(s)`), plus the usual weak CRT hooks
(`_ITM_*registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__`).

Rust `.so` imports the same `printf`/`puts` plus only libc / libgcc-unwind
symbols pulled in by the Rust standard library (`malloc`, `memcpy`, `mmap64`,
`_Unwind_*`, `pthread_key_*`, …).

**0 missing/undefined non-libc symbols in the Rust `.so`.**
