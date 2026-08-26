# SYMBOLS.md — Symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libdriver.so

# Rust
cargo build --no-default-features
nm -D --defined-only target/debug/libdriver.so
```

## Exported (defined) dynamic symbols

`c_src/CMakeLists.txt` compiles exactly one translation unit (`src/driver.c`)
into `libdriver.so`, and `include/driver.h` declares exactly one function.
There is no other C source file in `c_src/`, so there is no untranslated
module.

| # | C symbol | nm type (C) | Rust symbol | nm type (Rust) | status |
|---|----------|-------------|-------------|----------------|--------|
| 1 | `driver` | `T` (global text) | `driver` | `T` (global text) | ✅ present, exact name |

**Missing from Rust `.so`: 0.**

The Rust export is produced by `#[unsafe(no_mangle)] pub extern "C" fn driver(f: c_double)`
in `src/lib.rs`, so the differential tests exercise the real export wrapper.

There are no macro-generated symbols in the C source (the only preprocessor
directives are `#include` and the `DRIVER_H_` header guard — verified by
`grep -rn "ifdef\|ifndef\|if defined\|#else" c_src/`).

## Undefined (imported) symbols

| library | non-libc / non-runtime undefined symbols |
|---------|------------------------------------------|
| C `libdriver.so` | none (imports only `printf@GLIBC_2.2.5`, plus the weak `__cxa_finalize`, `__gmon_start__`, `_ITM_*TMCloneTable`) |
| Rust `libdriver.so` | none — every `U`/`w` entry resolves to glibc or libgcc |

The Rust `.so`'s extra undefined symbols are all pulled in by the Rust standard
library / panic runtime (`_Unwind_*` from `libgcc_s`, `malloc`, `memcpy`,
`dl_iterate_phdr`, `pthread_key_*`, …). `ldd` confirms the only dependencies are
`libgcc_s.so.1`, `libc.so.6`, and the loader:

```
libgcc_s.so.1 => /lib64/libgcc_s.so.1
libc.so.6 => /lib64/libc.so.6
```

Critically, the Rust `.so` imports the **same** `printf@GLIBC_2.2.5` that the C
`.so` imports. `src/lib.rs` forwards to the platform C library's `printf` with
an identical format string, so the `%llx`, `%a`, and `%.4f` conversions are
performed by the very same glibc code in both builds.

## Completion gate

- [x] `nm -D` shows **0** missing exported symbols in the Rust `.so`.
- [x] `nm -D` shows **0** undefined non-libc / non-runtime symbols in the Rust `.so`.
- [x] No C source file in `c_src/` was left untranslated (only `src/driver.c` exists).
