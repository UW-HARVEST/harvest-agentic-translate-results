# SYMBOLS.md — Public ABI surface parity (Phase A / Phase D)

Derived mechanically, not from assumptions:

```sh
# C shared object
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libtranslated_rust.so

# Rust shared object
cargo build --offline --no-default-features            # debug
cargo build --offline --release --no-default-features  # release
nm -D --defined-only target/{debug,release}/libhsv_to_rgb_lib.so
```

## C source inventory (completeness check)

Every file compiled into the C `.so` per `c_src/CMakeLists.txt`
(`add_library(${project_name} SHARED src/lib.c)`):

| C file | lines | translated to | status |
|--------|-------|---------------|--------|
| `c_src/src/lib.c` | 59 | `src/lib.rs` | fully translated |
| `c_src/include/lib.h` | 1 (decl only) | `src/lib.rs` (`extern "C"` signature) | n/a — header |

There are **no** untranslated C translation units, so there is no missing
implementation to back-fill (the Phase A "translate the missing C source" rule
does not apply here).

## Exported (defined, dynamic) symbols

`nm -D --defined-only` on the C `.so`:

```
0000000000001109 T hsv_to_rgb
```

That is the complete C export list — exactly one symbol.

| # | C symbol | kind | exported by Rust `.so`? | Rust definition |
|---|----------|------|-------------------------|-----------------|
| 1 | `hsv_to_rgb` | `T` (global text) | **YES** — `T hsv_to_rgb` in both `target/debug/libhsv_to_rgb_lib.so` and `target/release/libhsv_to_rgb_lib.so` | `src/lib.rs`, `#[unsafe(no_mangle)] pub unsafe extern "C" fn hsv_to_rgb` |

There are no macro-generated / aliased / versioned exports in the C source
(no `__attribute__((alias))`, no `.symver`, no `EXPORT(...)`-style macros —
verified by grep: the only non-static function definition in `c_src/src/lib.c`
is `hsv_to_rgb`).

## Symbol diff (Phase D gate)

```
comm -23 <C defined symbols> <Rust defined symbols>   ->  (empty)
```

* Symbols in C `.so` but **missing** from Rust `.so`: **0**
* Undefined (`U`) non-libc symbols in the Rust `.so`: **0**

Weak/loader-provided entries present in both objects and intentionally excluded
from the comparison because they are toolchain artifacts, not library API:
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__gmon_start__`,
`__cxa_finalize@GLIBC_2.2.5`.

Undefined imports differ only in that the C object imports `floorf@GLIBC_2.2.5`
while the Rust object inlines/lowers `f32::floor`; this is an *import*, not an
export, so it does not affect ABI parity. Behavioural equivalence of that
lowering is covered by the `CONFIGS.md` rows that drive `floorf` over normals,
subnormals, zeros, infinities and NaNs.

## Build configurations

`Cargo.toml` declares `[features] default = []` and no other features, and
`c_src/src/lib.c` / `c_src/include/lib.h` contain **no** `#if`/`#ifdef`/
`#ifndef` conditional compilation and `CMakeLists.txt` defines no
`target_compile_definitions`. Therefore the complete set of valid
feature combinations is:

| # | cargo invocation | notes |
|---|------------------|-------|
| 1 | `--no-default-features` (empty feature set) | identical to #2, `default = []` |
| 2 | `--all-features` / default | no features exist to enable |

Both are verified in `run_all.sh`, in both `debug` and `release` profiles
(the release profile matters: it is the one with `panic = "abort"` and without
Rust's debug-only pointer-UB assertions).
