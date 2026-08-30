# SYMBOLS.md — Phase A symbol map

Derived mechanically from `nm -D` on both shared libraries.

## C shared library

Built with:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
```

Artifact: `c_src/build/libharvest-work-DEAujn.so` (the CMake project name is derived
from the parent directory name, so the file name tracks the checkout directory).

`nm -D c_src/build/libharvest-work-DEAujn.so`:

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
00000000000024c8 T get_predict_func
```

## Rust shared library

Artifact: `translation/target/release/libget_predict_func_lib.so`

`nm -D --defined-only target/release/libget_predict_func_lib.so`:

```
00000000000120c0 T get_predict_func
```

## Parity table

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `get_predict_func` | `T` (defined) | `T` (defined) | the only public API symbol; declared in `c_src/include/lib.h` |

### Weak / toolchain symbols (not part of the API surface)

`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__` are weak undefined symbols emitted by the GCC/glibc CRT glue.
They are not definitions and not part of the library's API, so they are excluded
from the parity requirement.

### Missing-symbol analysis

**0 missing symbols.** The Rust `.so` exports every symbol the C `.so` defines,
with the exact same name.

## Internal (non-exported) C functions

Every other function in `c_src/src/lib.c` is declared `static`, i.e. it has
internal linkage and therefore appears in **neither** `.so`'s dynamic symbol
table. They are:

| C function | linkage | Rust counterpart | exported? |
|---|---|---|---|
| `BTAC1C2_PredictSample` | `static` | `BTAC1C2_PredictSample` (private) | no (correct) |
| `BTAC1C2_PredictSample_Pfn0` .. `_Pfn11` (12 fns) | `static` | same names (private) | no (correct) |
| `BTAC1C2_GetPredictFunc` | `static` | `BTAC1C2_GetPredictFunc` (private) | no (correct) |

These are **not** completeness failures: they are absent from the C `.so`'s
dynamic symbol table too, so exporting them from Rust would be a *divergence*.

They are still translated in `translation/src/lib.rs`, because
`get_predict_func`'s return value is derived from comparing their addresses, and
because their arithmetic is part of the translation unit's behaviour.

### How the internal functions are still differentially tested

`translation/src/lib.rs` gains four extra exports — `__difftest_predict`,
`__difftest_selector`, `__difftest_call_selected` and `__difftest_layout` — **only**
under the non-default `difftest` cargo feature. The test harness builds a
matching C shim (`translation/difftest_c/shim.c`) which `#include`s
`c_src/src/lib.c` verbatim (c_src itself is never modified) and exposes the same
`__difftest_predict` dispatcher. That lets all 13 internal predictors be compared
across the FFI boundary.

The default-feature Rust `.so` exports `get_predict_func` and nothing else, so
default-build parity with the C `.so` is exact.

## Feature combinations checked

| features | Rust exported symbols | matches C default surface |
|---|---|---|
| (default, none) | `get_predict_func` | yes — exact |
| `difftest` | `get_predict_func`, `__difftest_call_selected`, `__difftest_layout`, `__difftest_predict`, `__difftest_selector` | superset; the extra symbols are test-only and gated off by default |
