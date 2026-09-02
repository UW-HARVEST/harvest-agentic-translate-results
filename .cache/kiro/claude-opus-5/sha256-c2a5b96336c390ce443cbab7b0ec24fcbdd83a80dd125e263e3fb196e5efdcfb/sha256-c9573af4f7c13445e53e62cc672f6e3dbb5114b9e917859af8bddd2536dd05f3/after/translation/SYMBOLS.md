# SYMBOLS.md — Symbol parity: C `.so` vs Rust `.so`

Derived mechanically from `nm -D` on both shared objects.

## Source inventory (completeness check)

The C library is built from exactly the sources listed in `c_src/CMakeLists.txt`:

```
add_library(${project_name} SHARED
    src/lib.c)
```

Full C source tree (`find c_src -type f`, excluding `build/`):

| C file | translated? | Rust location |
|--------|-------------|---------------|
| `c_src/include/lib.h` | yes (declaration only) | `translation/src/lib.rs` |
| `c_src/src/lib.c` | yes (the single definition, `bin2hex`) | `translation/src/lib.rs` |

No C module is missing from the translation, so no Phase-A "translate the
skipped module" work is required. There are no namespacing/renaming
preprocessor macros in the header, so the linker symbol is plainly `bin2hex`.

## Commands

```sh
nm -D --defined-only c_src/build/libharvest-work-5FUsip.so
nm -D --defined-only translation/target/release/libbin2hex_lib.so
```

## Exported (defined, dynamic) symbols

### C `.so` — `libharvest-work-5FUsip.so`

| symbol | type | exported by Rust `.so`? |
|--------|------|--------------------------|
| `bin2hex` | `T` (global text) | **yes** — `#[unsafe(no_mangle)] pub unsafe extern "C" fn bin2hex` |

Total: **1** exported symbol.

### Rust `.so` — `libbin2hex_lib.so`

| symbol | type |
|--------|------|
| `bin2hex` | `T` (global text) |

Total: **1** exported symbol.

## Diff

```
C exported \ Rust exported  =  (empty)
Rust exported \ C exported  =  (empty)
```

**0 symbols missing from the Rust `.so`.** No stubs, no `unimplemented!()`;
`bin2hex` is a real translation of `c_src/src/lib.c`.

## Undefined (imported) symbols

The C `.so` imports only `abort@GLIBC_2.2.5` plus the standard weak
`_ITM_*` / `__cxa_finalize` / `__gmon_start__` entries.

The Rust `.so` imports the same `abort@GLIBC_2.2.5` plus libc/`libgcc`
runtime support pulled in by `std` (`malloc`, `memcpy`, `write`, `_Unwind_*`,
`pthread_key_*`, `dl_iterate_phdr`, …). **All Rust undefined symbols are libc,
libgcc-unwind, or standard weak ELF symbols — there are 0 undefined non-libc
symbols**, i.e. nothing the Rust `.so` expects another translation unit to
provide.

Verified by `tests/symbol_parity.rs::c_symbols_are_all_exported_by_rust`, which
re-runs `nm -D` on both objects at test time and fails on any diff, and by
`tests/symbol_parity.rs::rust_so_has_no_undefined_non_libc_symbols`.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section** and no optional
dependencies, so the only build configuration is the default one
(`--no-default-features` is equivalent to the default). See `CONFIGS.md`.
