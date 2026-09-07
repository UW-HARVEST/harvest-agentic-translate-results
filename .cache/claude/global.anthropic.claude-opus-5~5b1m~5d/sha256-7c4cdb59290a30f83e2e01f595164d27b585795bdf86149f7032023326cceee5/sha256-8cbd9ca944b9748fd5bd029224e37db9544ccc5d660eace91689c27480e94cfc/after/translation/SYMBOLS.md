# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared libraries.

## Build commands used

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

cd translation && cargo build --release
# -> translation/target/release/libdriver.so
```

## C source inventory (completeness check)

The entire C library is two files:

| file | contents |
|------|----------|
| `c_src/include/driver.h` | one declaration: `void driver(double f);` |
| `c_src/src/driver.c` | `typedef union { uint64_t x; double f; } raw_double_t;` (file-local) and the definition of `driver` |

`c_src/CMakeLists.txt` lists exactly one source file (`src/driver.c`) in the
`driver` SHARED target. There is **no** untranslated module: the Rust crate's
`src/lib.rs` covers 100% of the C translation units. `raw_double_t` is a
translation-unit-local typedef, so it contributes no linker symbol; it is
nevertheless mirrored in Rust as a `#[repr(C)] union` for fidelity.

## C `.so` exported symbols (`nm -D --defined-only`)

Filtering out the linker/loader-synthesised entries that CMake/GCC always emits
(`_init`, `_fini`, `__bss_start`, `_edata`, `_end`) leaves the library's real
public ABI:

| # | symbol | type | signature |
|---|--------|------|-----------|
| 1 | `driver` | `T` (global text) | `void driver(double)` |

Raw output:

```
0000000000001109 T driver
```

No preprocessor namespace/renaming macros exist in the header, so the linker
name is plainly `driver` — there are no macro-generated symbols to mirror.

## Rust `.so` exported symbols

Raw output, with Rust-mangled (`_ZN…`) internal std symbols filtered out:

```
00000000000116d0 T driver
```

`driver` is exported via `#[unsafe(no_mangle)] pub extern "C" fn driver(f: f64)`.

## Parity table

| # | symbol | in C `.so` | in Rust `.so` | status |
|---|--------|-----------|--------------|--------|
| 1 | `driver` | yes | yes | ✅ match |

**Missing from Rust: none.** No stubs, no `unimplemented!()`, no faked exports.

## Undefined (imported) symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` resolves entirely against libc /
libgcc / the Rust std that is statically linked into the `cdylib`:
`printf` (the same libc entry point the C library calls), plus the usual
`memcpy`, `__libc_start_main`-family, pthread and unwind imports std pulls in.

**0 missing/undefined non-libc symbols.** ✅

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
feature configuration is the empty default set. `--no-default-features` and
`--all-features` are therefore identical to the default build; Phases B and C
are still re-run under each spelling to prove it (see `run_all.sh`).

## Binary targets

Neither project builds an executable driver: `CMakeLists.txt` declares only
`add_library(driver SHARED …)` and the crate has no `[[bin]]` / `src/main.rs`.
The "compare C and Rust binary stdout" gate is therefore vacuous — but note
that `driver`'s entire observable effect *is* stdout, so every Phase B/C test
compares captured stdout byte-for-byte anyway.
