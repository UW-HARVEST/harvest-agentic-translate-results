# SYMBOLS.md — dynamic-symbol surface parity

Derived mechanically from `nm -D` on both shared objects.

## Commands used

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libharvest-work-IN8iuS.so

# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libcall_predict_lib.so
```

## C `.so` defined dynamic symbols

```
00000000000024c8 T call_predict
```

## C `.so` undefined dynamic symbols (all weak libc/toolchain, not translatable)

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
```

## Rust `.so` defined dynamic symbols

```
00000000000116a0 T call_predict
```

## Parity table

| # | C symbol | present in Rust `.so` | notes |
|---|----------|-----------------------|-------|
| 1 | `call_predict` | YES (`#[unsafe(no_mangle)] pub extern "C" fn`) | only exported entry point of the library |

**Symbol diff (C defined − Rust defined): EMPTY.**

## Symbols intentionally NOT exported

* `get_predict_func` — declared in the public header `c_src/include/lib.h`
  (`int get_predict_func(int pfcn);`) but **never defined anywhere in the C
  sources**. It does not appear in `nm -D` on the C `.so`, so it is not part of
  the ABI. Exporting it from Rust would *add* a symbol the C does not have, and
  any body would be a fabrication. Correctly absent.
* `BTAC1C2_PredictSample`, `BTAC1C2_PredictSample_Pfn0` … `_Pfn11`,
  `BTAC1C2_GetPredictFunc` — all declared `static` in `c_src/src/lib.c`, hence
  internal linkage and absent from the dynamic symbol table. They are
  nevertheless fully translated in `translation/src/lib.rs` (as private
  `unsafe extern "C" fn` items) because `call_predict` observes their
  *addresses*; no module or function of the C source was skipped.

## Completeness check of the translation vs. the C source

Every top-level definition in `c_src/src/lib.c` has a Rust counterpart:

| C definition | kind | Rust counterpart |
|---|---|---|
| `btac1c_u16` / `btac1c_s16` / `btac1c_byte` | typedef | `type btac1c_u16/_s16/_byte` |
| `struct btac1c_idxstate_s` | struct | `#[repr(C)] pub struct btac1c_idxstate` |
| `BTAC1C2_PredictSample` | static fn | `BTAC1C2_PredictSample` |
| `BTAC1C2_PredictSample_Pfn0..11` | static fn ×12 | `BTAC1C2_PredictSample_Pfn0..11` |
| `BTAC1C2_GetPredictFunc` | static fn | `BTAC1C2_GetPredictFunc` |
| `call_predict` | exported fn | `call_predict` |

Nothing missing, nothing stubbed, no `unimplemented!()`.
