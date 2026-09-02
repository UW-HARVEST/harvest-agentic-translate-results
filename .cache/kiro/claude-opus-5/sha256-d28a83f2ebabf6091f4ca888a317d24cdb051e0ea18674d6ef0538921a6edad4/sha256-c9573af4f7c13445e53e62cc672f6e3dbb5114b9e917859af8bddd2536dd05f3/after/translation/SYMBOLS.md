# SYMBOLS.md — exported-symbol parity between the C `.so` and the Rust `.so`

Derived mechanically. Commands used:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
nm -D --undefined-only translation/target/release/libdriver.so
```

## C translation units

The whole C library is a single translation unit: `c_src/src/driver.c`
(`c_src/CMakeLists.txt` → `add_library(driver SHARED src/driver.c)`).
No other C source file exists, so no module could have been skipped by the
translation. Verified with `find c_src -name '*.c' -o -name '*.h'`:

* `c_src/include/driver.h`
* `c_src/src/driver.c`

## Defined (exported) symbols

| # | symbol | in C `.so` | in Rust `.so` | declared in `driver.h`? | notes |
|---|--------|-----------|---------------|-------------------------|-------|
| 1 | `driver` | `T` (0x1176) | `T` | yes | `void driver(const char *in)` |
| 2 | `foo`    | `T` (0x1129) | `T` | **no** | `int foo(const char *in, char c)` — no `static`, so external linkage and part of the ABI. Exported from Rust with `#[unsafe(no_mangle)] extern "C"`. |

No macro-generated symbols exist in this library (grep for `#define` in
`c_src` finds only the `DRIVER_H_` include guard).

**Missing from Rust `.so`: none.** Symbol diff is empty in both directions for
`T`/`D`/`B` (defined-global) symbols.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind
imports (`printf`, `malloc`, `memcpy`, `strlen`, `_Unwind_*`, `__errno_location`,
…). **0 missing/undefined non-libc symbols.** The only one that matters
semantically is `printf@GLIBC_2.2.5`, which the Rust `driver` deliberately
reuses so that formatting *and* stdio buffering are byte-identical to the C
library.

## ABI notes confirmed by disassembly

* C `foo` stores only `%al` of the second argument and then sign-extends it
  (`mov %esi,%eax; mov %al,-0x1c(%rbp); movsbl -0x1c(%rbp),%edx`), i.e. only
  the **low byte** of `c` is significant.
* Rust `foo` compares with `cmp %sil,%cl`, i.e. also only the low byte.
* Therefore garbage in the upper 24 bits of the second argument register must
  be ignored identically by both. This is covered by a differential test
  (`ERRORS.md` row 8).
