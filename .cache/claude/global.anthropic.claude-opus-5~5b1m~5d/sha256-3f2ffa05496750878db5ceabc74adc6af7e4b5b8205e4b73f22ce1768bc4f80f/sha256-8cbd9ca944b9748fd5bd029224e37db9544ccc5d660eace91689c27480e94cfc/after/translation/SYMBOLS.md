# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-jbdC4v.so` (built via CMakeLists.txt)
* Rust `.so`: `translation/target/release/libget_predict_func_lib.so`
  (`[lib] name = "get_predict_func_lib"`, `crate-type = ["cdylib"]`)

## Defined dynamic symbols

```
$ nm -D --defined-only c_src/build/libharvest-work-jbdC4v.so
00000000000024c8 T get_predict_func

$ nm -D --defined-only translation/target/release/libget_predict_func_lib.so
00000000000120c0 T get_predict_func
```

| # | C symbol | type | exported by Rust `.so`? | notes |
|---|----------|------|-------------------------|-------|
| 1 | `get_predict_func` | `T` (global text) | **YES** — exact name | `int get_predict_func(int pfcn)`, the only symbol in `include/lib.h`. Rust: `#[unsafe(no_mangle)] pub extern "C" fn get_predict_func`. |

**Missing symbols: NONE.** The symbol diff (C-defined minus Rust-defined) is empty.

## Why the C `.so` exports only one symbol

Every other function in `c_src/src/lib.c` has C internal linkage (`static`) and
is therefore *deliberately* absent from the dynamic symbol table:

| C function | linkage | translated in Rust? | Rust item |
|------------|---------|---------------------|-----------|
| `BTAC1C2_PredictSample`       | `static` | yes | `BTAC1C2_PredictSample` (private) |
| `BTAC1C2_PredictSample_Pfn0`  | `static` | yes | `BTAC1C2_PredictSample_Pfn0` (private) |
| `BTAC1C2_PredictSample_Pfn1`  | `static` | yes | `BTAC1C2_PredictSample_Pfn1` (private) |
| `BTAC1C2_PredictSample_Pfn2`  | `static` | yes | `BTAC1C2_PredictSample_Pfn2` (private) |
| `BTAC1C2_PredictSample_Pfn3`  | `static` | yes | `BTAC1C2_PredictSample_Pfn3` (private) |
| `BTAC1C2_PredictSample_Pfn4`  | `static` | yes | `BTAC1C2_PredictSample_Pfn4` (private) |
| `BTAC1C2_PredictSample_Pfn5`  | `static` | yes | `BTAC1C2_PredictSample_Pfn5` (private) |
| `BTAC1C2_PredictSample_Pfn6`  | `static` | yes | `BTAC1C2_PredictSample_Pfn6` (private) |
| `BTAC1C2_PredictSample_Pfn7`  | `static` | yes | `BTAC1C2_PredictSample_Pfn7` (private) |
| `BTAC1C2_PredictSample_Pfn8`  | `static` | yes | `BTAC1C2_PredictSample_Pfn8` (private) |
| `BTAC1C2_PredictSample_Pfn9`  | `static` | yes | `BTAC1C2_PredictSample_Pfn9` (private) |
| `BTAC1C2_PredictSample_Pfn10` | `static` | yes | `BTAC1C2_PredictSample_Pfn10` (private) |
| `BTAC1C2_PredictSample_Pfn11` | `static` | yes | `BTAC1C2_PredictSample_Pfn11` (private) |
| `BTAC1C2_GetPredictFunc`      | `static` | yes | `BTAC1C2_GetPredictFunc` (private) |

Exporting any of these from the Rust `.so` would be an ABI *divergence*, so they
stay private.  They are still differentially tested: see
`tests/difftest_c_harness.c`, a test-only harness that lives OUTSIDE `c_src/`,
`#include`s `c_src/src/lib.c` verbatim, and re-exports the internal predictors
through `__difftest_predict`.  The Rust crate exposes the mirror-image hook
`__difftest_predict` behind the `difftest` Cargo feature.

## Non-default feature builds

| build | Rust-defined dynamic symbols | C symbols missing |
|-------|------------------------------|-------------------|
| `--no-default-features` (== default; there are no default features) | `get_predict_func` | none |
| `--features difftest` | `get_predict_func`, `__difftest_predict` | none |

`__difftest_predict` is a superset symbol used only by the test harness; the
shipped default build does not contain it, so the production ABI is identical.

## Undefined (imported) symbols

Both libraries import only libc / unwinder symbols.  The C `.so` imports 4 weak
CRT symbols; the Rust `.so` additionally imports glibc + `_Unwind_*` symbols
pulled in by the Rust runtime (`malloc`, `memcpy`, `abort`, `dl_iterate_phdr`,
…).  **0 missing / undefined non-libc symbols in the Rust `.so`.**
