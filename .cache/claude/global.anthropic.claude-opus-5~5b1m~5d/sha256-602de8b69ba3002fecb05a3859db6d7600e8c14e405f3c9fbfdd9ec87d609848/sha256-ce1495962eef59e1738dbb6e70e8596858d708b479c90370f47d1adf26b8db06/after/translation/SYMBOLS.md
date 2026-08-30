# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
#  -> c_src/build/libSimpleList.so
cd translation && cargo build --release
#  -> translation/target/release/libSimpleList.so
```

## Defined (exported) dynamic symbols

`nm -D --defined-only` on each `.so`:

| # | symbol | type | in C `.so` | in Rust `.so` | status |
|---|--------|------|-----------|--------------|--------|
| 1 | `smallestValue` | `T` (global text) | yes | yes | MATCH |

C:
```
00000000000010f9 T smallestValue
```
Rust:
```
00000000000116a0 T smallestValue
```

**Symbol diff (C exported − Rust exported): EMPTY.** No symbol is missing from
the Rust `.so`, so no `#[no_mangle]` wrapper had to be added and no C module was
left untranslated. The C library consists of exactly one translation unit
(`src/simplestruct.c`, 38 lines) declaring exactly one public function in
`include/simplestruct.h`; `translation/src/lib.rs` covers it in full.

There are no macro-generated symbols in this library (the only preprocessor
directives are the `SIMPLESTRUCT_H_` include guard and the `#include`).

## Undefined (imported) symbols

| library | undefined non-libc symbols |
|---------|---------------------------|
| C       | none (only weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__`) |
| Rust    | none (all are libc / `_Unwind_*` / glibc pthread+TLS runtime imports pulled in by the Rust std prelude) |

Rust's extra imports (`malloc`, `memcpy`, `_Unwind_*`, `dl_iterate_phdr`, …)
come from linking Rust `std`, not from unresolved translation references. They
are all satisfied by libc/libgcc at load time, which the differential tests
confirm by successfully `dlopen`-ing the Rust `.so`.

## Data symbols / types

`struct ListNode` is a header-only type; it produces no dynamic symbol. Its ABI
is still part of the surface and is verified explicitly:

| field | C offset | Rust offset |
|-------|----------|-------------|
| `int value` | 0 | 0 |
| `struct ListNode *next` | 8 | 8 |
| `sizeof` / `align` | 16 / 8 | 16 / 8 |

Verified at runtime by `tests/layout.rs` (compiles a throwaway C probe that
prints `sizeof`/`offsetof` and compares against Rust's `size_of`/`offset_of!`).

## Gate status

- [x] `nm -D` shows **0 missing** symbols in the Rust `.so` relative to the C `.so`.
- [x] `nm -D` shows **0 undefined non-libc** symbols in the Rust `.so`.
