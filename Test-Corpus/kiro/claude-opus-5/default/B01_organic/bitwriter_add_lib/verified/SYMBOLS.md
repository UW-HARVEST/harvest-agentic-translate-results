# SYMBOLS.md — Public ABI surface parity

Derived mechanically from `nm -D` on both shared objects.

```
C   .so : c_src/build/libharvest-work-gmUtdR.so
Rust.so : translation/target/release/libbitwriter_add_lib.so
```

## C `.so` defined dynamic symbols (`nm -D --defined-only`)

| # | symbol | type | present in Rust `.so`? |
|---|--------|------|------------------------|
| 1 | `bitwriter_add` | `T` (global text) | YES — `#[unsafe(no_mangle)] pub unsafe extern "C" fn bitwriter_add` |

Total C exported symbols: **1**. Total matched in Rust: **1**.

## Symbol diff

```
$ comm -23 <(nm -D --defined-only C.so   | awk '{print $NF}' | sort) \
           <(nm -D --defined-only RUST.so| awk '{print $NF}' | sort)
(empty)
```

**0 symbols missing from the Rust `.so`.**

Note: the header `include/lib.h` contains no namespace-renaming preprocessor
macros and no macro-generated function definitions, so the set of linker names
is exactly the set of source-level function names. `c_src/src/lib.c` is the only
translation unit in `c_src/CMakeLists.txt`, and it defines exactly one function.
No C module was skipped by the translation.

## Undefined symbols

The C `.so` has only weak ABI/CRT stubs undefined
(`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

The Rust `.so` additionally references libc and libgcc unwinder symbols
(`malloc`, `memcpy`, `_Unwind_*`, `pthread_key_create`, ...) pulled in by the
Rust standard library / panic machinery. **0 non-libc, non-unwinder undefined
symbols.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
buildable configuration is the default (empty) feature set. Verified with:

```
$ grep -n '^\[features\]' Cargo.toml   # no match
```

Consequently `cargo test`, `cargo test --no-default-features`, and
`cargo test --all-features` are the same configuration; all three are run by
`check_features.sh`.
