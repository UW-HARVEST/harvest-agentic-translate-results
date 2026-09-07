# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-hItD9B.so` (from `c_src/CMakeLists.txt`,
  which compiles exactly one translation unit: `src/lib.c`)
* Rust `.so`: `translation/target/release/libdiv_euclid_lib.so`
  (`crate-type = ["cdylib"]`, `name = "div_euclid_lib"`)

## C `.so` — defined dynamic symbols (`nm -D --defined-only`)

| # | symbol | type | declared in | exported by Rust `.so`? |
|---|--------|------|-------------|-------------------------|
| 1 | `div_euclid` | `T` (global text) | `c_src/include/lib.h:1` — `int div_euclid(int v1, int v2);` | YES (`0000000000011690 T div_euclid`) |

`c_src/include/lib.h` is a single line and declares no other entry point. There
are no macros in the C source that generate additional symbol names, no
`#ifdef`-guarded extra definitions, and no second translation unit — so the
source-level name is the final linker name and the table above is complete.

## Undefined / weak symbols

The C `.so` additionally references only the standard GCC/glibc startup weak
symbols (`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`). The Rust `.so` references those
same weak symbols plus `U` entries that all come from the Rust standard library
runtime (`_Unwind_*` from libgcc, and libc entries such as `malloc`, `memcpy`,
`open64`, `pthread_key_create`, `write`). None of them is a symbol that the C
library was expected to provide.

## Diff

```
$ comm -23 <(nm -D --defined-only c_src/build/*.so       | awk '{print $NF}' | sort -u) \
           <(nm -D --defined-only translation/target/release/libdiv_euclid_lib.so \
                                                          | awk '{print $NF}' | sort -u)
(empty)
```

**Symbols exported by the C `.so` but missing from the Rust `.so`: 0.**
**Non-libc undefined symbols in the Rust `.so`: 0.**

No wrapper had to be added and no C module was left untranslated: `src/lib.c`
contains one function and `translation/src/lib.rs` translates it with
`#[unsafe(no_mangle)] pub extern "C" fn div_euclid`.
