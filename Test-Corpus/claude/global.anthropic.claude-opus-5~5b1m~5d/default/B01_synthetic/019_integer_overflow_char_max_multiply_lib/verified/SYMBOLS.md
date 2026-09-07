# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared libraries.

- C:    `c_src/build/libdriver.so`
- Rust: `translation/target/release/libdriver.so`

## Defined (exported) symbols

| # | symbol | C `.so` | Rust `.so` | C signature | notes |
|---|--------|---------|------------|-------------|-------|
| 1 | `bad`              | T | T | `void bad(void)`                  | exported, non-static |
| 2 | `driver`           | T | T | `void driver(int useGood)`         | the only symbol in `driver.h` |
| 3 | `good`             | T | T | `void good(void)`                 | exported, non-static |
| 4 | `printHexCharLine` | T | T | `void printHexCharLine(char)`     | exported, non-static |
| 5 | `printLine`        | T | T | `void printLine(const char *)`    | exported, non-static |

**Symbol diff (C defined \ Rust defined): EMPTY.**
**Symbol diff (Rust defined \ C defined): EMPTY.**

## Deliberately NOT exported

These are `static` in `c_src/src/driver.c`, so they have internal linkage and do
not appear in `nm -D` for the C `.so`. The Rust translation likewise keeps them
private (plain `unsafe fn`, no `#[no_mangle]`), which is correct parity:

| C symbol | linkage in C | Rust |
|----------|--------------|------|
| `goodG2B` | `static void goodG2B()` | private `unsafe fn goodG2B()` |
| `goodB2G` | `static void goodB2G()` | private `unsafe fn goodB2G()` |

## Undefined (imported) symbols

The C `.so` imports only `printf` and `puts` from glibc (gcc rewrites
`printf("%s\n", line)` into `puts(line)`, which is byte-identical output).

The Rust `.so` imports `printf`, `puts` plus the usual Rust runtime set
(`_Unwind_*`, `malloc`/`free`/`realloc`/`calloc`, `memcpy`, `write`, ...).
All Rust-side undefined symbols are libc / libgcc_s runtime symbols — there are
**0 missing or undefined non-libc symbols**.

## Verification command

```sh
diff <(nm -D --defined-only c_src/build/libdriver.so            | awk '{print $NF}' | sort) \
     <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
```

Result: no output (identical sets). Asserted by the test
`symbols::rust_so_exports_every_c_symbol`.
