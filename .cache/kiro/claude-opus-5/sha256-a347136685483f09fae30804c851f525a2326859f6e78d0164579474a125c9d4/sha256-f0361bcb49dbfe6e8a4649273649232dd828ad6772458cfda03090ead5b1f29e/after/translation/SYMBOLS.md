# Phase A.1 — Symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-G26WQf.so

cd translation && cargo build --release
# -> translation/target/release/libto_barycentric_lib.so
```

## C `.so` — `nm -D --defined-only`

```
00000000000011a3 T to_barycentric
```

Only one defined dynamic symbol exists besides the linker/CRT boilerplate that
`nm -D --defined-only` already excludes (`_init`, `_fini`, `__bss_start`,
`_edata`, `_end` are section markers emitted by the linker, not translation
units). The three helpers in `c_src/src/lib.c` — `lm_v2`, `lm_sub2`, `lm_dot2` —
are declared `static`, so they have internal linkage and are deliberately absent
from the dynamic symbol table:

```
$ nm libharvest-work-G26WQf.so | grep -E ' t | T '
0000000000001173 t lm_dot2
0000000000001126 t lm_sub2
00000000000010f9 t lm_v2
00000000000011a3 T to_barycentric
(plus CRT boilerplate: _init, _fini, frame_dummy, register_tm_clones,
 deregister_tm_clones, __do_global_dtors_aux)
```

## Rust `.so` — `nm -D --defined-only`

```
0000000000011690 T to_barycentric
```

## Parity table

| # | symbol | in C `.so` | in Rust `.so` | status |
|---|--------|-----------|---------------|--------|
| 1 | `to_barycentric` | T (global) | T (global) | MATCH |

There are no macro-generated symbols in this library (the C source contains no
preprocessor definitions other than `#include "lib.h"`), and no whole C module
was skipped by the translation: `c_src/src/lib.c` is the only translation unit
listed in `c_src/CMakeLists.txt`, and every one of its four functions has a
counterpart in `translation/src/lib.rs`:

| C function | linkage | Rust counterpart |
|---|---|---|
| `lm_v2` | `static` | `lm_v2` (portable path); on x86-64 the packing is the closing `unpcklps` of `to_barycentric_core` |
| `lm_sub2` | `static` | the three `subss` pairs in `to_barycentric_core`; `lm_sub2` on the portable path |
| `lm_dot2` | `static` | the five `mulss`/`mulss`/`addss` triples in `to_barycentric_core`; `lm_dot2` on the portable path |
| `to_barycentric` | global | `#[unsafe(no_mangle)] pub extern "C" fn to_barycentric` |

The Rust `.so` must also not export *more* than the C does. The arithmetic core
`to_barycentric_core` is declared `.hidden` in the `global_asm!` block for
exactly this reason, mirroring the `static` linkage of the C helpers; it is
absent from `nm -D` and the test `symbol_parity::asm_core_is_not_exported`
enforces that.

## Automated gate

The symbol comparison is not a one-off shell command — it is a test:

* `symbol_parity::symbol_sets_are_identical` — runs `nm -D --defined-only` on
  both `.so` files, strips the linker-synthesised section markers (`_init`,
  `_fini`, `__bss_start`, `_edata`, `_end`, `__TMC_END__`), and asserts the two
  sets are **equal** (no missing symbols *and* no extra ones).
* `symbol_parity::no_undefined_non_libc_symbols` — runs `nm -D -u` on the Rust
  `.so` and asserts that every undefined symbol is either version-tagged
  (`sym@GLIBC_x.y`, `sym@GCC_x.y`) or one of the three weak toolchain hooks, so
  no untranslated library symbol can be left dangling.
* `symbol_parity::asm_core_is_not_exported` — as above.

## Undefined (imported) symbols

C `.so`, `nm -D -u` — four weak toolchain hooks only, no libc dependency; the
object needs no libc at all and `.rodata` holds a single `1.0f` constant
(`2000 0000803f`):

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
```

Rust `.so`, `nm -D -u` — the same four weak hooks plus 45 entries that are all
`libc` / `libgcc_s` symbols pulled in by the Rust runtime shim (`memcpy`,
`malloc`, `abort`, `pthread_key_*`, `_Unwind_*`, `dl_iterate_phdr`,
`__errno_location`, …). None of them is a library symbol that the translation
failed to provide.

## Gate

- [x] `nm -D` shows **0** missing symbols in the Rust `.so` relative to the C `.so`.
- [x] `nm -D -u` shows **0** undefined non-libc symbols in the Rust `.so`.
