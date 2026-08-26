# SYMBOLS.md — Symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

```
C   .so: c_src/build/libtranslated_rust.so   (cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON; cmake --build .)
Rust.so: target/{debug,release}/libnormalize_lib.so   (cargo build [--release])
```

## C source inventory (completeness check)

`c_src/` contains exactly one translation unit and one public header, so no C
module can have been skipped by the translation step:

| C file | public entry points | translated in |
|--------|--------------------|---------------|
| `c_src/src/lib.c` | `normalize` | `src/lib.rs` |
| `c_src/include/lib.h` | declaration of `normalize` only | `src/lib.rs` doc header |

`nm -A --defined-only` on the C object confirms `normalize` is the only
non-libc definition; there are no static helpers, no macro-generated symbol
families, no tables, and no `#ifdef`-selected alternates in the C source.

## Exported (defined, dynamic) symbols

`nm -D --defined-only` output, verbatim symbol names:

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `normalize` | `T` | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn normalize` |

**Missing from Rust `.so`: 0.**
**Extra Rust-only exported symbols: 0** (the cdylib exports no `rust_eh_*`,
no `__rust_*` allocator shims in its dynamic *defined* symbol table beyond
`normalize`).

## Undefined (imported) symbols

The C `.so` imports `memset@GLIBC`, `sqrtf@GLIBC` plus the four standard weak
CRT hooks (`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`).

The Rust `.so` imports the same `memset@GLIBC` (used by
`core::ptr::write_bytes`) and resolves `sqrtf` inline as the `sqrtss`
instruction (`f32::sqrt` → `llvm.sqrt.f32`), so `sqrtf` does not appear as an
import. Every remaining Rust import is libc / libgcc-unwind runtime
(`_Unwind_*`, `malloc`, `mmap64`, `pthread_*`, `dl_iterate_phdr`, …) pulled in
by `std`; **0 undefined non-libc symbols**.

Verified automatically by `tests/symbol_parity.rs`
(`c_defined_symbols ⊆ rust_defined_symbols`, and no non-libc undefined symbol
remains in the Rust `.so`).

## Build configurations

`Cargo.toml` declares `[features] default = []` and no other feature, so the
**complete** set of feature combinations is:

| # | combination | command |
|---|-------------|---------|
| 1 | *(none — `default` is empty)* | `cargo check/test --no-default-features` |

`c_src/CMakeLists.txt` declares no `option()`, no `add_definitions`, no
`CMAKE_BUILD_TYPE`-dependent branch and no `#ifdef` in the source, so the C
library likewise has exactly one configuration.

Both Rust build profiles (`dev` and `release`) are nevertheless exercised,
because `opt-level` can change floating-point code generation
(vectorisation / instruction selection) even though it must not change results.
