# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Derived mechanically, not from assumptions:

```
# C side
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libharvest-work-ZOtKwo.so

# Rust side
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libencode_quant_lib.so
```

## C translation-unit inventory (completeness check)

`CMakeLists.txt` compiles exactly one source file into the shared library:

```cmake
add_library(${project_name} SHARED
    src/lib.c)
```

`find c_src -type f` yields only `CMakeLists.txt`, `include/lib.h`, `src/lib.c`.
There is **no untranslated module**: `src/lib.c` (62 lines, one function) is
fully translated in `translation/src/lib.rs`. The public header declares exactly
one prototype and contains **no** namespace-renaming / symbol-generating macros:

```c
int encode_quant(int uni, int step, int pred, int tgt, int tgt2, int lsbit);
```

## Symbol table

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `encode_quant` | `T` (text, global) | `T` (text, global) | **PRESENT in both** |

Raw output:

```
=== C defined dynamic symbols ===
00000000000010f9 T encode_quant

=== Rust defined dynamic symbols ===
00000000000116d0 T encode_quant
```

## Symbol diff

```
$ diff <(nm -D --defined-only C.so   | awk '{print $NF}' | sort) \
       <(nm -D --defined-only RUST.so | awk '{print $NF}' | sort)
(empty)
```

**Symbols exported by C but missing from Rust: 0.**
No `#[no_mangle]` wrapper had to be added and no C module had to be translated;
the surface was already complete. (No stubs / `unimplemented!()` exist anywhere
in the crate — verified with `grep -rn 'unimplemented!\|todo!\|panic!' src/`.)

## Undefined-symbol audit of the Rust `.so`

`nm -D -u translation/target/release/libencode_quant_lib.so` lists only
platform runtime imports — glibc (`malloc`, `memcpy`, `open64`, `pthread_*`,
`__errno_location`, …), the libgcc unwinder (`_Unwind_*`), and weak ELF
housekeeping symbols (`__gmon_start__`, `_ITM_*`, `statx`, `gettid`).

**Missing/undefined non-libc symbols: 0.** ✅

## Feature-combination matrix

`translation/Cargo.toml` declares **no `[features]` table**, so the crate has
exactly one configuration: the default (empty) feature set. Phase D's
"every feature combination" therefore collapses to a single combo, and it is
still exercised explicitly across all three of
`--no-default-features`, default, and `--all-features`
(see `check_all_feature_combos.sh`).
