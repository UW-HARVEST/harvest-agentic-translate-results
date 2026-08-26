# SYMBOLS.md — Phase A.1: exported-symbol parity

Mechanically derived from `nm -D` on both shared libraries. Nothing here is
assumed; every row is copied from the tool output reproduced below.

## Build commands

```sh
# C shared library
cd translated_rust/c_src && mkdir -p build && cd build
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
#   -> c_src/build/libtranslated_rust.so

# Rust shared library (crate-type = ["cdylib"], lib name = max_size_frame_lib)
cd translated_rust && cargo build
#   -> target/debug/libmax_size_frame_lib.so
```

## Raw `nm -D --defined-only` output

C library (`c_src/build/libtranslated_rust.so`):

```
00000000000010f9 T max_size_frame
```

Rust library (`target/debug/libmax_size_frame_lib.so`):

```
0000000000011dc0 T max_size_frame
```

## Symbol table

| # | C symbol (`nm -D`) | type | exported by Rust `.so`? | Rust item |
|---|--------------------|------|-------------------------|-----------|
| 1 | `max_size_frame`   | `T` (global text) | YES — exact name match | `#[unsafe(no_mangle)] pub extern "C" fn max_size_frame` in `src/lib.rs` |

**Missing symbols: 0.** No `#[no_mangle]` wrapper had to be added and no C
source file was left untranslated.

## Completeness audit of the C source

The entire C library is two files, reproduced in full so the audit is verifiable:

* `c_src/include/lib.h` (5 lines) — `#include <stdint.h>`, one typedef
  (`typedef uint32_t tflac_u32;`) and one function declaration. A `typedef`
  emits no symbol, so the header contributes exactly one symbol.
* `c_src/src/lib.c` (10 lines) — the single definition of `max_size_frame`.
  There are no `static` helpers, no additional translation units, no global
  variables, and no macro-generated symbol families (verified: `grep -nE
  '[*]|enum|struct|union' src/lib.c include/lib.h` matches only multiplication
  operators, and `CMakeLists.txt` lists `src/lib.c` as the only source).

Therefore the complete public ABI surface is the single function above, and the
Rust translation covers 100 % of it.

## Undefined (imported) symbols

C library:

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
```

All four are weak toolchain/libc runtime hooks emitted by GCC's CRT glue, not
library API. `max_size_frame` is a leaf function that calls nothing, so there
are **0 missing/undefined non-libc symbols** on either side.

## Verification checklist

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
- [x] Every C-exported symbol is exported by the Rust `.so` with the exact name.
- [x] The symbol diff (C set minus Rust set) is **empty**.
- [x] No symbol is stubbed, faked, or `unimplemented!()` — the one export is a
      real translation of the C body.

The diff is produced automatically by `tests/symbol_parity.rs` and by
`run_all_tests.sh`, so it is re-checked on every test run rather than trusted
from this document.
