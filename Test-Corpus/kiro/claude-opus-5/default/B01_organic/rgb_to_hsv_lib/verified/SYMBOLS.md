# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-TGRcUc.so

cd translation && cargo build --release
# -> translation/target/release/librgb_to_hsv_lib.so
```

## C source inventory

The whole C library is a single translation unit:

| C file | public declarations |
|--------|---------------------|
| `c_src/src/lib.c` | `rgb_to_hsv` (the only function defined in the file) |
| `c_src/include/lib.h` | `void rgb_to_hsv(float *dest, const float *src);` (the entire header, 1 line) |

No other `.c`/`.h` files exist, so no C module was skipped by the translation.

## Defined dynamic symbols (`nm -D --defined-only`)

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `rgb_to_hsv` | `T` | `T` | PRESENT in both — exact name match |

C `.so` defined-symbol count: 1
Rust `.so` defined-symbol count: 1
Symbol diff (C \ Rust): **empty**

The Rust side exports it via `#[unsafe(no_mangle)] pub unsafe extern "C" fn rgb_to_hsv`,
so the export wrapper itself is what the differential tests exercise (they always
`libloading::Library::get` the symbol out of `librgb_to_hsv_lib.so`; no Rust
function is ever called directly).

## Undefined dynamic symbols

C `.so`: only weak toolchain/libc symbols
(`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`).

Rust `.so`: only libc + libgcc-unwind symbols
(`memcpy`, `malloc`, `free`, `abort`, `_Unwind_*`, `pthread_key_*`, `mmap64`, …)
pulled in by the Rust standard library / panic runtime.

**0 missing / undefined non-libc symbols on the Rust side.**

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table, so there is exactly one
build configuration. `--no-default-features` and `--all-features` are therefore
identical to the default build; both are still exercised in the test matrix
script (`run_all.sh`) for completeness.

## Binary targets

Neither project builds an executable driver: `c_src/CMakeLists.txt` contains only
`add_library(... SHARED src/lib.c)` (no `add_executable`), and
`translation/Cargo.toml` declares only `[lib] crate-type = ["cdylib"]`
(no `[[bin]]`, no `src/main.rs`). The "compare binary stdout" clause of the
completion gate is therefore not applicable.
