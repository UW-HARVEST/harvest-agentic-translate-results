# SYMBOLS.md — Symbol parity between C `.so` and Rust `.so`

Derived mechanically from:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory (completeness check)

`c_src` contains exactly two files of interest:

| C file | functions defined |
|--------|-------------------|
| `c_src/include/driver.h` | declares `driver` only |
| `c_src/src/driver.c` | `printLine`, `printIntLine`, `bad`, `good`, `driver` |

All 5 functions defined in `driver.c` have external linkage (no `static`), so all
5 land in the dynamic symbol table. There is **no untranslated C module** — the
whole library is a single translation unit and `translation/src/lib.rs` covers
every function in it. Nothing is stubbed.

## Defined dynamic symbols

| # | symbol | C `.so` | Rust `.so` | C signature | notes |
|---|--------|---------|------------|-------------|-------|
| 1 | `printLine`    | `T` | `T` | `void printLine(const char *line)` | NULL-guarded |
| 2 | `printIntLine` | `T` | `T` | `void printIntLine(int intNumber)`  | |
| 3 | `bad`          | `T` | `T` | `void bad(void)`  | CWE-482, result of `intOne + intTwo` discarded |
| 4 | `good`         | `T` | `T` | `void good(void)` | |
| 5 | `driver`       | `T` | `T` | `void driver(void)` | calls all of the above |

**Symbol diff (C defined − Rust defined): EMPTY.**
**Symbol diff (Rust defined − C defined): EMPTY.** (Rust exports no extra
non-libc symbols; `crate-type = ["cdylib"]` with `-C prefer-dynamic=no` keeps the
Rust runtime symbols internal.)

## Undefined (imported) symbols

| symbol | C `.so` | Rust `.so` | comment |
|--------|---------|------------|---------|
| `printf@GLIBC_2.2.5` | `U` | `U` | libc |
| `puts@GLIBC_2.2.5`   | `U` | (not required) | GCC rewrites `printf("%s\n", s)` → `puts(s)`; a pure libc optimisation with byte-identical output. Not a behavioural difference. |
| `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__` | `w` | — | weak toolchain/CRT symbols, not part of the API surface |

All remaining undefined symbols in the Rust `.so` are libc / `ld.so` symbols
(`memcpy`, `__libc_start_main`, `pthread_*`, `dl_iterate_phdr`, …) pulled in by
the Rust standard library. **0 missing/undefined non-libc symbols.**

## Cargo feature surface

`translation/Cargo.toml` declares **no `[features]` table**, therefore the
complete set of feature combinations is:

| # | combination | cargo invocation |
|---|-------------|------------------|
| 1 | (default = empty) | `cargo test` |
| 2 | (no-default-features = empty, identical to #1) | `cargo test --no-default-features` |

Both are verified in Phase D.
