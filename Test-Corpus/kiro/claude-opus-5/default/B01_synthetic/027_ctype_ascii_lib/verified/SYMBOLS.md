# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

## C source inventory (completeness check)

The whole C subtree is two files; there is no untranslated module.

```
$ find c_src -name '*.c' -o -name '*.h'
c_src/include/driver.h      -> declares: void driver(char c);
c_src/src/driver.c          -> defines:  void driver(char c);
```

`driver.c` defines exactly one function (`driver`) and no file-scope
variables. All other identifiers it references (`setlocale`, `printf`, and the
`<ctype.h>` classifiers, which are glibc macros over
`__ctype_b_loc` / `__ctype_tolower_loc` / `__ctype_toupper_loc`) are libc
imports, not exports of this library.

## `nm -D --defined-only` — C `.so`

```
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001149 T driver
```

Non-libc, non-toolchain defined symbols: **1** (`driver`).

## `nm -D --defined-only` — Rust `.so`

```
$ nm -D --defined-only translation/target/release/libdriver.so
00000000000126e0 T driver
```

## Parity table

| # | C symbol | type | exported by Rust `.so`? | note |
|---|----------|------|-------------------------|------|
| 1 | `driver` | `T` (global text) | YES (`T driver`) | `#[unsafe(no_mangle)] pub extern "C" fn driver(c: c_char)` |

Missing from Rust: **none**. No `#[no_mangle]` wrapper had to be added and no
C module had to be translated — the single translation unit is fully covered.

## Undefined (imported) symbols

Rust `.so` undefined symbols must be libc-only. Verified with
`nm -D --undefined-only`; the entries are the glibc ctype/locale/stdio imports
plus the usual `GLIBC_*` / `__cxa_*` / unwinder plumbing. No unresolved
project symbols.

Checklist:

- [x] `nm -D` shows 0 missing symbols in the Rust `.so` relative to the C `.so`.
- [x] 0 undefined non-libc symbols in the Rust `.so`.

## Signature note (not a symbol-parity issue)

The C prototype is `void driver(char c)`; the Rust export is declared
`pub extern "C" fn driver(arg: c_int)` and truncates with `arg as c_char`. The
exported *symbol* is identical (`T driver`) and the two are
ABI-indistinguishable on SysV x86-64, where a sub-word argument travels in the
low bits of the argument register with the upper bits unspecified. Declaring the
Rust parameter `c_char` made rustc tag it `signext`, which let LLVM keep an
out-of-range value a promoted C caller had passed and read past the end of the
ctype table; see the row-6 write-up in `ERRORS.md`. Verified with a real gcc
consumer compiled against the unmodified header in
`scripts/compare_binaries.sh`.

## Automated check

`scripts/symbol_diff.sh` is the authoritative version of this file: it runs
`nm -D` on both objects, strips toolchain-generated entries (`_init`, `_fini`,
`__bss_start`, `_edata`, `_end`, `_ITM_*`, `__cxa_*`, `__gmon_start__`), diffs
the two sets, and additionally asserts the Rust `.so` has no undefined symbol
outside libc / the language runtime. Current output:

```
C .so defined symbols   (1):
  driver
Rust .so defined symbols (1):
  driver
symbol diff: EMPTY (Rust exports every C symbol)
undefined symbols: all libc/runtime (51 entries)
```
