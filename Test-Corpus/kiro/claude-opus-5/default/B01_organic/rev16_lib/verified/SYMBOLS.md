# SYMBOLS.md — Public ABI surface parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-BoQyTH.so

cd translation && cargo build --release
# -> translation/target/release/librev16_lib.so
```

## C source inventory (completeness check)

The entire C library is two files:

| file | lines | function definitions |
|------|-------|----------------------|
| `c_src/include/lib.h` | 3 | (declaration only) `uint32_t rev16(uint32_t a);` |
| `c_src/src/lib.c` | 9 | `rev16` |

`c_src/CMakeLists.txt` compiles exactly one translation unit (`src/lib.c`) into
the shared library. There is no second module, no conditional compilation, and
no macro-generated symbol family. Therefore no C source was left untranslated:
the single function `rev16` is fully implemented in `translation/src/lib.rs`.

## Exported (defined, dynamic) symbols

`nm -D --defined-only`:

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `rev16` | `T rev16` | `T rev16` | PRESENT in both — exact name match |

Symbol diff (C exported minus Rust exported): **empty**.

```
$ comm -23 <(nm -D --defined-only c_src/build/libharvest-work-BoQyTH.so | awk '{print $NF}' | sort) \
           <(nm -D --defined-only translation/target/release/librev16_lib.so | awk '{print $NF}' | sort)
(no output)
```

The Rust `.so` exports no *extra* non-libc public symbols either; `rev16` is the
only `T` entry in both objects.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust object lists only libc / dynamic-loader /
libgcc-unwind imports pulled in by the Rust standard library
(`malloc`, `memcpy`, `__errno_location`, `_Unwind_*`, `dl_iterate_phdr`,
`pthread_key_*`, …). Every one resolves from `libc.so.6` / `libgcc_s.so.1`.

**0 missing or undefined non-libc symbols.**

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table, so the only build
configuration is the default one. `--no-default-features` is therefore
equivalent to the default build; both are exercised in Phase D.
