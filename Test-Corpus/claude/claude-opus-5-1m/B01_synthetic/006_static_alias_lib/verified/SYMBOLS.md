# SYMBOLS.md — Symbol parity: C `.so` vs Rust `.so`

Derived mechanically from `nm -D`.

* C   : `c_src/build/libStaticAlias.so`  (cmake, default config)
* Rust: `target/debug/libStaticAlias.so` (`crate-type = ["cdylib"]`, `[lib] name = "StaticAlias"`)

## Build-time configurations

`Cargo.toml` has **no `[features]` section**, and `c_src/CMakeLists.txt` declares no
options / `add_definitions` / `#ifdef`-driven variants (the only `#ifdef` in the C
tree is the `STATICALIAS_H_` include guard). There is therefore exactly **ONE**
valid feature combination:

| # | combination | `cargo check --no-default-features --features <combo>` |
|---|-------------|--------------------------------------------------------|
| 1 | *(empty — no features)* | pass (clean, 0 errors, 0 warnings) |

`--all-features` is equivalent to the empty set here.

## Exported (defined) dynamic symbols

`nm -D --defined-only` on the C `.so` yields exactly two symbols. Both are
exported by the Rust `.so` under the exact same names:

| # | C symbol | C type | signature (from `include/staticalias.h`) | in Rust `.so`? | Rust item |
|---|----------|--------|------------------------------------------|----------------|-----------|
| 1 | `static_alias` | `T` (global text) | `int *static_alias(int *outer);` | YES — `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn static_alias` |
| 2 | `driver`       | `T` (global text) | `void driver(int initial_value, int iterations);` | YES — `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` |

**Missing symbols: 0.** No C source file was left untranslated: `src/staticalias.c`
is the only translation unit in `CMakeLists.txt`, and both of its non-static
functions are present and exported. There are no macro-generated exports, no
exported data objects, and no `static`/internal C functions to account for.

The C `.so` exports no data symbols (`static int inner` is function-local and has
no dynamic symbol); correspondingly the Rust `INNER` static is private and must
*not* appear in `nm -D`, which matches.

## Undefined (imported) symbols

| library | undefined non-libc symbols |
|---------|----------------------------|
| C | none (`printf` + weak `__cxa_finalize`/`__gmon_start__`/`_ITM_*` only) |
| Rust | none — all imports are libc (`printf`, `malloc`, `memcpy`, …), the weak `_ITM_*`/`__gmon_start__` stubs, and `_Unwind_*` from `libgcc_s`, all of which are satisfied at load time |

Both `.so`s import `printf@GLIBC_2.2.5`, so `driver`'s output goes through the
*same* libc `stdout` FILE in a process that loads both — which the tests rely on
when capturing bytes.

**Gate: `nm -D` shows 0 missing symbols in the Rust `.so` and 0 unresolved
non-libc undefined symbols. PASS.**
