# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# Rust
cargo build --offline            # target/debug/libdriver.so
```

## Source inventory

The C library is a single translation unit — `c_src/src/driver.c` (58 lines) with
one public header, `c_src/include/driver.h`. `CMakeLists.txt` lists exactly one
source file:

```cmake
add_library(driver SHARED src/driver.c)
```

There is therefore **no untranslated C module**: `src/lib.rs` covers the whole
library. Every function defined in `driver.c` is accounted for below.

## Symbol table

`nm -D --defined-only` on each `.so`, restricted to the library's own symbols
(the Rust `.so` additionally exports the usual `_ZN…`/`_R…` Rust-internal and
`std` symbols, which have no C counterpart and are not part of the ABI surface):

| # | symbol | C signature | in C `.so` | in Rust `.so` | Rust item |
|---|--------|-------------|-----------|---------------|-----------|
| 1 | `printIntPtrLine` | `void printIntPtrLine(const int *intNumber)` | `T` | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn printIntPtrLine` |
| 2 | `bad`             | `void bad(void)`                             | `T` | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn bad` |
| 3 | `good`            | `void good(void)`                            | `T` | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn good` |
| 4 | `driver`          | `void driver(int useGood)`                    | `T` | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` |

Only `driver` is declared in the public header; `printIntPtrLine`, `bad` and
`good` are non-`static` in `driver.c` and so are exported too. All four are part
of the ABI surface and all four are tested.

`frame_body` (the shared stack-frame helper in `src/lib.rs`) is deliberately
**not** `#[no_mangle]`/`pub` — it has no counterpart in the C `.so` and adding an
export would break parity in the other direction.

## Parity check

```sh
nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort > c_syms.txt
nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort > r_syms.txt
comm -23 c_syms.txt r_syms.txt      # symbols in C but missing from Rust
```

Result: **empty** — 4/4 symbols present, exact-name match, nothing stubbed.

## Undefined (imported) symbols

`nm -D --undefined-only`:

* C `.so`: `printf@GLIBC_2.2.5` plus the standard weak
  `_ITM_*` / `__cxa_finalize` / `__gmon_start__` entries.
* Rust `.so`: the same `printf@GLIBC_2.2.5` — `src/lib.rs` imports the C
  library's `printf` by `#[link_name = "printf"]` rather than using Rust
  formatting, so number formatting and stdio buffering are byte-identical —
  plus libc (`malloc`, `memcpy`, `write`, …) and the libgcc unwinder
  (`_Unwind_*`) pulled in by the Rust runtime.

There are **0 missing / undefined non-libc symbols** in the Rust `.so`:
`ldd target/debug/libdriver.so` resolves fully against `libgcc_s.so.1` and
`libc.so.6`.

## Verification status

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so`, exact name.
- [x] `comm -23` symbol diff is empty.
- [x] No symbol is stubbed, faked, or `unimplemented!()`.
- [x] No C source file was left untranslated.
- [x] 0 unresolved non-libc undefined symbols.
