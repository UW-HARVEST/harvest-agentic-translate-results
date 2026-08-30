# SYMBOLS.md — exported-symbol parity

Source of truth: `nm -D --defined-only` on the C shared library
`c_src/build/libdriver.so`, compared against `translation/target/release/libdriver.so`.

Regenerate with:

```sh
nm -D --defined-only c_src/build/libdriver.so       | awk '{print $3}' | sort > /tmp/c.syms
nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms   # must be EMPTY
```

## Defined (exported) symbols

| # | symbol | C `.so` | Rust `.so` | C signature | notes |
|---|--------|---------|------------|-------------|-------|
| 1 | `printLine`    | T | T | `void printLine(const char *line)` | NULL-guarded `printf("%s\n", line)` |
| 2 | `printIntLine` | T | T | `void printIntLine(int intNumber)`  | `printf("%d\n", intNumber)` |
| 3 | `bad`          | T | T | `void bad(void)`                    | `alloca(10)` under-allocation (CWE-806) |
| 4 | `good`         | T | T | `void good(void)`                  | `alloca(10*sizeof(int))` |
| 5 | `driver`       | T | T | `void driver(int useGood)`          | only symbol declared in `include/driver.h` |

**Missing from Rust `.so`: none.** The whole C translation unit (`c_src/src/driver.c`,
the project's only source file) is translated in `src/lib.rs`; there is no skipped
module and no stubbed symbol.

Note: in the optimized Rust build the linker/ICF folds `bad` and `good` onto the
same address because after translation both have identical observable behaviour
(copy ten zeroed `int`s, then `printIntLine(0)`). Both names are still exported
and both are individually resolvable via `dlsym`, which is what the differential
tests assert.

## Undefined (imported) symbols

The C `.so` imports only `printf`, `puts` (gcc's `printf("%s\n",x)` →
`puts(x)` transform) plus the usual weak CRT hooks.

The Rust `.so` imports the same `printf`/`puts` plus libc/`libgcc` runtime
symbols pulled in by the Rust standard library (`malloc`, `memcpy`, `mmap64`,
`_Unwind_*`, `pthread_key_*`, …).

**Non-libc / non-runtime undefined symbols in the Rust `.so`: 0.** Every
undefined entry resolves out of `libc.so.6` / `libgcc_s.so.1`, so the Rust
`.so` loads with no unresolved dependency:

```sh
nm -D -u translation/target/release/libdriver.so   # all glibc/GCC_* versioned or weak CRT
```
