# SYMBOLS.md — Phase A symbol surface

Mechanically derived from `nm -D` on both shared objects.

Build commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-HYhLMf.so   (target name == parent dir name, see CMakeLists.txt)

# Rust
cd translation && cargo build --offline && cargo build --release --offline
# -> translation/target/{debug,release}/libhsv_to_rgb_lib.so
```

## C translation unit inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

| C source file | translated to | status |
|---|---|---|
| `c_src/src/lib.c` (59 lines, 1 function) | `translation/src/lib.rs` | fully translated |
| `c_src/include/lib.h` (1 line, 1 declaration) | `translation/src/lib.rs` (`extern "C"` signature) | fully translated |

There is no second module / file that was skipped, so no "absent implementation"
case (Phase A rule 2) applies here.

## `nm -D --defined-only` — exported (T) symbols

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `hsv_to_rgb` | `T` @ 0x1109 | `T` @ 0x11cb0 (release) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` |

**Diff of exported symbol sets: EMPTY.** (`comm -23` of the two sorted `T`-symbol
lists produces no lines — see `tests/phase_d_parity.rs::exported_symbol_sets_match`,
which recomputes this at test time instead of trusting this file.)

The C `.so` exports no data symbols, no macro-generated symbols and no
`__attribute__((alias))` symbols, so `hsv_to_rgb` is the entire public ABI.

## Non-exported dynamic symbols (informational)

C `.so`, full `nm -D`:

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
                 U floorf@GLIBC_2.2.5
0000000000001109 T hsv_to_rgb
```

Rust `.so` undefined (`nm -D -u`) symbols are all libc / libgcc-unwind
imports (`floorf`, `memcpy`, `malloc`, `_Unwind_*`, `dl_iterate_phdr`, ...)
pulled in by `std`'s panic/backtrace machinery. Both objects import `floorf`
from glibc, so `floorf` behaviour is *identical by construction* (same
implementation is called by both).

Checklist:

- [x] every `T` symbol of the C `.so` is exported by the Rust `.so` with the
      exact same name
- [x] 0 missing symbols
- [x] 0 undefined non-libc symbols in the Rust `.so`
- [x] no stubbed / `unimplemented!()` symbol was added to fake parity
