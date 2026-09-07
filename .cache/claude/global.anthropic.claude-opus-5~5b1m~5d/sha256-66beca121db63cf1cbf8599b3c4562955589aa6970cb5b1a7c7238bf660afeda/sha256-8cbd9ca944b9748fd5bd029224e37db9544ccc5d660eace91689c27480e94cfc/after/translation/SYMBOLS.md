# SYMBOLS.md — Phase A symbol surface

## C shared library

Built from `c_src/CMakeLists.txt` (project name is derived from the parent
directory name, hence `libharvest-work-6iQDjX.so`).

```
$ nm -D --defined-only c_src/build/libharvest-work-6iQDjX.so
00000000000024c8 T call_predict
```

Exactly **one** dynamic, defined, global symbol: `call_predict`.

`c_src/include/lib.h` declares `int get_predict_func(int pfcn);` but the C
source **never defines it**, so it is not (and must not be) an exported symbol.

## Rust shared library

```
$ nm -D --defined-only translation/target/release/libcall_predict_lib.so | grep -v ' [wt] '
00000000000116a0 T call_predict
```

## Parity table

| # | C symbol | type | present in Rust `.so` | notes |
|---|----------|------|-----------------------|-------|
| 1 | `call_predict` | `T` (global text) | YES (`T call_predict`) | `#[unsafe(no_mangle)] pub extern "C" fn call_predict(pfcn: c_int) -> c_int` |

**Symbol diff (C exports minus Rust exports): EMPTY.**

## Internal (non-exported) C functions — translated but intentionally not exported

These are `static` in C, therefore have local (`t`) linkage and are *not* part
of the dynamic symbol table. They are still translated because their
**addresses** are observable through `call_predict`.

| C static function | Rust counterpart |
|-------------------|------------------|
| `BTAC1C2_PredictSample` | `BTAC1C2_PredictSample` |
| `BTAC1C2_PredictSample_Pfn0` .. `_Pfn11` (12 fns) | `BTAC1C2_PredictSample_Pfn0` .. `_Pfn11` |
| `BTAC1C2_GetPredictFunc` | `BTAC1C2_GetPredictFunc` |

Total: 14 static helpers, all present in `translation/src/lib.rs`.

## Undefined symbols in the Rust `.so`

All remaining undefined dynamic symbols are libc / Rust-runtime imports
(`memcpy`, `__cxa_thread_atexit_impl`, `pthread_*`, etc.). There are **no
undefined non-libc symbols** that the C library resolves and the Rust one does
not.

## Test-only `internal_probe` feature

`Cargo.toml` gains an **off-by-default** feature `internal_probe` that exports
thin `probe_*` wrappers around the 14 helpers that are `static` in C, so their
arithmetic can be differentially tested (see `CONFIGS.md` rows 21–28). It only
**adds** symbols — it never removes, renames or stubs one — so the parity table
above still holds:

```
$ comm -23 <(syms C.so) <(syms rust-default.so)      # C-only symbols
(empty)
$ comm -23 <(syms C.so) <(syms rust-internal_probe.so)
(empty)
```

The feature is not enabled for any shipped artifact; the default build exports
exactly `call_predict` and nothing else, matching the C `.so` byte for byte in
its dynamic symbol table.

**No symbol required translating from scratch** — `c_src/src/lib.c` is the only
C source file in the project (`c_src/CMakeLists.txt` lists just `src/lib.c`) and
every function in it has a Rust counterpart. There are no stubs and no
`unimplemented!()` anywhere in `translation/src/lib.rs`.
