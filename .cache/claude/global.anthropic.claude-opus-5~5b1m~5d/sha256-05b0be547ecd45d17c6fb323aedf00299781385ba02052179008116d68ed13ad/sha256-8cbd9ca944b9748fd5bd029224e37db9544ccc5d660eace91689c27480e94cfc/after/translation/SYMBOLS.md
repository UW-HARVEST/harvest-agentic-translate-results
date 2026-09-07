# SYMBOLS.md — Phase A / Phase D symbol surface

Mechanically derived from `nm -D` on both shared libraries.

Commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libharvest-work-isR6ra.so

# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libbin2hex_lib.so
```

## Defined (exported) dynamic symbols

| # | C symbol (`nm -D --defined-only`) | type | present in Rust `.so` | Rust item |
|---|-----------------------------------|------|-----------------------|-----------|
| 1 | `bin2hex`                         | `T`  | YES (`T bin2hex`)     | `#[no_mangle] pub unsafe extern "C" fn bin2hex` in `src/lib.rs` |

The C translation unit is a single file (`c_src/src/lib.c`, 27 lines) with a
single public header (`c_src/include/lib.h`, 5 lines) declaring exactly one
function. There are no namespace/renaming macros, no macro-generated symbol
families, no `#ifdef`-gated extra modules, and no additional `.c` files in
`c_src/CMakeLists.txt` (`add_library(... SHARED src/lib.c)`). Therefore the
complete public surface is the single symbol above — no C source was left
untranslated.

## Symbol diff

```
comm -23 <(nm -D --defined-only C.so   | awk '{print $NF}' | sort) \
         <(nm -D --defined-only RUST.so | awk '{print $NF}' | sort)
=> (empty)
```

**C-exported symbols missing from the Rust `.so`: 0.**

## Undefined (imported) symbols

The C `.so` imports only `abort@GLIBC` plus the standard weak
`_ITM_*` / `__cxa_finalize` / `__gmon_start__` glibc/ELF boilerplate.

The Rust `.so` imports `abort@GLIBC` (the real libc `abort`, declared in
`extern "C"` in `src/lib.rs`, so the abort path is observably identical:
`SIGABRT`) plus libc/`_Unwind_*` symbols pulled in by the Rust standard
library runtime (`malloc`, `memcpy`, `write`, `dl_iterate_phdr`, ...).

**Undefined non-libc / non-runtime symbols in the Rust `.so`: 0.**
All Rust `U` entries resolve against glibc or libgcc, which the loader
provides; `libloading` loads the Rust `.so` successfully (see
`tests/differential.rs`).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default one. Verified mechanically:

```sh
grep -n '^\[features\]' translation/Cargo.toml   # no match
```

`cargo check --no-default-features` and `cargo check` are therefore the same
build; both are run by `run_all.sh`.

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(... SHARED ...)` — there is no
`add_executable`, and `translation/Cargo.toml` declares only `[lib]` with
`crate-type = ["cdylib"]` and has no `src/main.rs` / `[[bin]]`. There is no
driver binary, so the "compare binary stdout" gate is not applicable.
