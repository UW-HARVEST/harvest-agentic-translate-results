# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C:    `c_src/build/libdriver.so`      (cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON)
- Rust: `translation/target/release/libdriver.so` (`cargo build --release`)

Regenerate / re-verify with:

```sh
nm -D --defined-only c_src/build/libdriver.so        | awk '{print $3}' | sort > /tmp/c.syms
nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms   # must print nothing (C symbols missing from Rust)
```

## Symbol table

| # | C symbol | C source (`c_src/src/driver.c`) | in C `.so` | in Rust `.so` | Rust impl | status |
|---|----------|---------------------------------|-----------|--------------|-----------|--------|
| 1 | `printLine`    | line 30 `void printLine(const char * line)` | T | T | `#[unsafe(no_mangle)] pub unsafe extern "C" fn printLine` | OK |
| 2 | `printIntLine` | line 38 `void printIntLine(int intNumber)`   | T | T | `#[unsafe(no_mangle)] pub extern "C" fn printIntLine`      | OK |
| 3 | `bad`          | line 43 `void bad()`                        | T | T | `#[unsafe(no_mangle)] pub extern "C" fn bad`               | OK |
| 4 | `good`         | line 57 `void good()`                       | T | T | `#[unsafe(no_mangle)] pub extern "C" fn good`              | OK |
| 5 | `driver`       | line 73 `void driver(int useGood)` (only symbol declared in `include/driver.h`) | T | T | `#[unsafe(no_mangle)] pub extern "C" fn driver` | OK |

There are no macro-generated symbols in this library: `c_src/src/driver.c`
contains no function-defining macros, and `include/driver.h` declares only
`driver`. `printLine`, `printIntLine`, `bad` and `good` have external linkage
in the C file (no `static`), so they are part of the exported ABI even though
they are not declared in the public header — the Rust crate exports all four.

**Missing symbols: none.** No module of `c_src/` was left untranslated
(`c_src/` contains exactly one translation unit, `src/driver.c`), so no
Phase-A "translate the missing C source" work was required.

## Undefined symbols (imports)

| `.so` | non-libc undefined symbols |
|-------|----------------------------|
| C     | none — imports only `printf`, `puts` (`@GLIBC_2.2.5`) plus the usual weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__` |
| Rust  | none — imports only glibc (`printf`, `puts`, `malloc`, `memcpy`, `write`, …) and the libgcc unwinder (`_Unwind_*@GCC_*`), which the Rust runtime pulls in |

`_Unwind_*` come from `libgcc_s`, part of the platform C runtime, and are
resolved at load time; `nm -D --undefined-only` on the Rust `.so` shows **0
missing/undefined non-libc symbols**. Verified: the test suite `dlopen`s the
Rust `.so` with `RTLD_NOW`, which fails outright on an unresolvable symbol.

## Note on `printf` vs `puts`

GCC rewrites `printf("%s\n", line)` into `puts(line)`, which is why the C `.so`
imports `puts`. The Rust translation calls `printf` directly. Both emit the
identical byte sequence (`puts` appends exactly one `\n`), and the differential
tests in `tests/differential.rs` compare captured `stdout` byte-for-byte, so
this implementation difference is covered rather than assumed.
