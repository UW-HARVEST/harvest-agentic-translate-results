# SYMBOLS.md — Symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```
# C
cd translated_rust/c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so

# Rust
cargo build --no-default-features
# -> target/debug/libget_predict_func_lib.so
```

## Defined (exported) symbols

`nm -D --defined-only`:

| # | symbol | in C `.so` | in Rust `.so` | notes |
|---|--------|-----------|---------------|-------|
| 1 | `get_predict_func` | ✅ `T` | ✅ `T` | the library's sole public symbol (`c_src/include/lib.h`) |

**Symbol diff: EMPTY.** Every symbol the C `.so` exports is exported by the
Rust `.so` under the exact same name, and the Rust `.so` exports no extra
public API symbols.

### Why there is only one symbol

`c_src/include/lib.h` declares exactly one function:

```c
int get_predict_func(int pfcn);
```

Every other function in `c_src/src/lib.c` is declared `static` (internal
linkage) and therefore is deliberately **not** exported:

| C function | linkage | exported? |
|------------|---------|-----------|
| `BTAC1C2_PredictSample` | `static` | no |
| `BTAC1C2_PredictSample_Pfn0` .. `_Pfn11` (12 fns) | `static` | no |
| `BTAC1C2_GetPredictFunc` | `static` | no |
| `get_predict_func` | external | **yes** |

The Rust translation mirrors this exactly: only `get_predict_func` carries
`#[no_mangle] pub extern "C"`; the 14 internal routines are private `unsafe
extern "C" fn` / `fn` items. Adding `#[no_mangle]` to any of them would be a
*divergence* from the C ABI surface, not a fix.

No module or C source file was skipped — `c_src/src/lib.c` (273 lines) is the
only translation unit in `CMakeLists.txt`, and all 15 of its functions are
present in `src/lib.rs`.

## Undefined symbols

`nm -D --undefined-only`:

* C `.so`: 4 weak runtime symbols (`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).
* Rust `.so`: the same 4, plus **only** libc (`malloc`, `memcpy`, `write`,
  `open64`, …) and the `_Unwind_*` / `dl_iterate_phdr` unwinder-and-backtrace
  imports pulled in by Rust `std`'s panic machinery.

**0 missing / undefined non-libc symbols in the Rust `.so`.**

## Feature combinations

`Cargo.toml` has **no `[features]` table**, and `grep -rn 'cfg(feature' src/`
returns nothing. There is therefore exactly **one** valid build configuration
(the default, identical to `--no-default-features`), and it is the
configuration verified in Phases B–D.
