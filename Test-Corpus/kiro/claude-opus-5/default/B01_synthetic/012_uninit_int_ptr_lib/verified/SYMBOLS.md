# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## Full C source inventory (completeness check)

`c_src/CMakeLists.txt` declares exactly one target:
`add_library(driver SHARED src/driver.c)`. There is **no** `add_executable`, so
the project builds no driver binary. `src/driver.c` is the only translation unit
and defines exactly four functions (`grep -nE '^void' c_src/src/driver.c`):

| C source file | function | translated in Rust? |
|---|---|---|
| `src/driver.c:28` | `printIntPtrLine` | yes — `translation/src/lib.rs` |
| `src/driver.c:33` | `bad` | yes — `translation/src/lib.rs` |
| `src/driver.c:39` | `good` | yes — `translation/src/lib.rs` |
| `src/driver.c:48` | `driver` | yes — `translation/src/lib.rs` |

No C module or file was skipped by the translation, so no missing-module
translation work was required under the Phase A rule.

`include/driver.h` declares only `driver`. The other three are non-`static` and
are therefore still exported and part of the public ABI. There are no
namespace-renaming or symbol-generating macros anywhere in the header or source,
so source-level names are the final linker names.

## Defined dynamic symbols

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `bad`             | T | T | MATCH |
| 2 | `driver`          | T | T | MATCH |
| 3 | `good`            | T | T | MATCH |
| 4 | `printIntPtrLine` | T | T | MATCH |

* Symbol diff (C-defined minus Rust-defined): **empty**.
* Reverse diff (extra exports in Rust): **empty**.
* Asserted by `sym_01_defined_symbol_parity`, and by `verify.sh`.

Presence in `.dynsym` is necessary but not sufficient, so
`sym_02_all_symbols_resolvable_via_dlsym` additionally resolves all four names
through `dlsym` in both libraries — which is also how the whole differential
suite calls them.

## Undefined (imported) symbols

| library | imports |
|---|---|
| C | `printf@GLIBC_2.2.5`; weak `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__` |
| Rust | the same set |

**0 missing symbols. 0 undefined non-libc symbols in the Rust `.so`.**

## Instruction-stream equivalence

Symbol parity says nothing about behaviour, and for this library the *frame
geometry* of each function is observable (see `ERRORS.md` row 7). `verify.sh`
therefore also diffs the normalised disassembly of each exported function:

| function | instructions | C vs Rust |
|---|---|---|
| `printIntPtrLine` | 14 | identical |
| `bad` | 9 | identical |
| `good` | 12 | identical |
| `driver` | 14 | identical |

## ELF profile

The two libraries must also present the same profile to the dynamic loader,
because the loader's lazy-resolution stack footprint is observable through
`bad()`. See the findings section of `CONFIGS.md`.

| library | `NEEDED` | `.dynsym` entries | `BIND_NOW` |
|---|---|---|---|
| C | 1 (`libc.so.6`) | 43 | no |
| Rust | 1 (`libc.so.6`) | 44 | no |

Asserted by `elf_02_dependency_parity`, `elf_03_dynsym_size_comparable` and
`link_mode_matches_c`.
