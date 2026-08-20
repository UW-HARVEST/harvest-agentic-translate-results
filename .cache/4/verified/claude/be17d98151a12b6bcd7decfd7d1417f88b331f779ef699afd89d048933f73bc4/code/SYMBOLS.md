# SYMBOLS.md — dynamic-symbol parity, C `.so` vs Rust `.so`

Derived mechanically:

```sh
# C reference library (exactly the command from the task description)
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so

cargo build                          # -> target/debug/libpinflate_lib.so

nm -D --defined-only c_src/build/libtranslated_rust.so | awk '$2!~/[wWuU]/{print $3}' | sort
nm -D --defined-only target/debug/libpinflate_lib.so   | awk '$2!~/[wWuU]/{print $3}' | sort
```

The whole C library is a single translation unit, `c_src/src/lib.c`
(`c_src/CMakeLists.txt` lists only `src/lib.c`), and its only header,
`c_src/include/lib.h`, declares exactly one function. Everything else that is
externally visible is a non-`static` global — those are part of the ABI too and
are checked here.

## Exported symbol table

| # | symbol | C type / signature | size (bytes) | ELF type | ELF bind | in C `.so` | in Rust `.so` |
|---|--------|--------------------|--------------|----------|----------|-----------|---------------|
| 1 | `pinflate`             | `int pinflate(void *in, int in_bytes, void *out, int out_bytes)` | – (FUNC) | FUNC   | GLOBAL | ✅ `T` | ✅ `T` |
| 2 | `cp_error_reason`      | `const char *`      | 8   | OBJECT | GLOBAL | ✅ `B` | ✅ `B` |
| 3 | `cp_fixed_table`       | `uint8_t [288+32]`  | 320 | OBJECT | GLOBAL | ✅ `D` | ✅ `D` |
| 4 | `cp_permutation_order` | `uint8_t [19]`      | 19  | OBJECT | GLOBAL | ✅ `D` | ✅ `D` |
| 5 | `cp_len_extra_bits`    | `uint8_t [29+2]`    | 31  | OBJECT | GLOBAL | ✅ `D` | ✅ `D` |
| 6 | `cp_len_base`          | `uint32_t[29+2]`    | 124 | OBJECT | GLOBAL | ✅ `D` | ✅ `D` |
| 7 | `cp_dist_extra_bits`   | `uint8_t [30+2]`    | 32  | OBJECT | GLOBAL | ✅ `D` | ✅ `D` |
| 8 | `cp_dist_base`         | `uint32_t[30+2]`    | 128 | OBJECT | GLOBAL | ✅ `D` | ✅ `D` |

`readelf -sW` reports identical `st_size`, `st_info` type and binding for all
eight symbols in both libraries (see the table's size/type/bind columns), so the
Rust `.so` is ABI-compatible and not merely name-compatible: a caller may
`dlsym()` any table and index it with the same bounds, and may write through it
(all seven objects are mutable in C and are `static mut` in Rust).

## Symbol diff

```
$ comm -23 csyms.txt rsyms.txt      # C symbols missing from Rust
(empty)
$ nm -D --undefined-only target/debug/libpinflate_lib.so | grep -v 'GLIBC\|GCC\|_ITM_\|__gmon\|__cxa'
(empty)
```

**C count: 8 — Rust count: 8 — missing: 0 — undefined non-libc in Rust: 0.**

## Symbols deliberately *not* exported

These exist in `c_src/src/lib.c` but are `static` (or unused), so they are not
part of the dynamic symbol table and must **not** appear in the Rust `.so`
either. They are translated as private Rust `fn`s:

`cp_make_pixel_a`, `cp_make_pixel`, `cp_would_overflow`, `cp_ptr`,
`cp_peak_bits`, `cp_consume_bits`, `cp_read_bits`, `cp_rev16`, `cp_build`,
`cp_stored`, `cp_fixed`, `cp_decode`, `cp_dynamic`, `cp_block`.

The C types `struct cp_pixel_t` / `struct cp_image_t` and the helpers
`cp_make_pixel_a` / `cp_make_pixel` are dead code in the C original (nothing
references them); they are carried over as `#[allow(dead_code)]` items so the
translation stays complete.

## Undefined (imported) symbols

The C library imports `calloc`, `free`, `memcpy`, `memset` and `__assert_fail`
from libc. The Rust library imports the equivalent allocator/`memcpy`/`memset`
routines plus its own `std` machinery; the presence of `__assert_fail` in the C
`.so` is what proves the reference build has **asserts enabled** (no
`-DNDEBUG`, because `c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`). See
`ERRORS.md` rows E7–E16: those asserts are observable behaviour and the Rust
port reproduces them as `abort()` (`SIGABRT`).

## Automated checks

`tests/phase_d_symbols.rs` re-derives this file from the binaries on every run,
so it cannot drift:

| test | what it enforces |
|------|------------------|
| `d1_every_c_symbol_is_exported_by_rust` | `nm -D --defined-only` set difference C → Rust is **empty** |
| `d2_symbol_set_is_exactly_the_c_source_surface` | the C `.so`'s exported set is exactly the 8 names above, and the Rust `.so` exports all 8 |
| `d3_static_functions_are_not_exported` | none of the 14 `static` C functions leaked into either dynamic symbol table |
| `d4_symbol_sizes_types_and_bindings_match` | `readelf -sW` `st_size`, ELF type and binding agree for all 8 (so a caller indexing an exported table cannot go out of bounds on one library but not the other) |
| `d5_rust_has_no_undefined_non_libc_symbols` | the Rust `.so` has no unresolved non-libc imports |
| `d6_exported_table_contents_are_byte_identical` | all 654 bytes of table **data** (320+19+31+124+32+128) behind the 6 array symbols are byte-identical, read through `dlsym` from both libraries |
| `d7_no_feature_flags_exist` | `Cargo.toml` has no `[features]` and `c_src/CMakeLists.txt` no `option()`/`CMAKE_BUILD_TYPE`, so the build matrix really is a single configuration — and `__assert_fail` is still present in the C `.so`, i.e. the asserts in `ERRORS.md` rows E7–E16 are still live |

## Nothing was stubbed

Every symbol is backed by a real translation of the corresponding C code; there
is no `unimplemented!()`, `todo!()`, `panic!("stub")` or empty body anywhere in
`src/lib.rs`. The whole C translation unit is covered:

```
$ grep -cE 'unimplemented!|todo!|unreachable!\(\)' src/lib.rs
0
```

`src/lib.rs` translates all 377 lines of `c_src/src/lib.c`, including the two
dead helpers (`cp_make_pixel_a`, `cp_make_pixel`) and the two dead types
(`cp_pixel_t`, `cp_image_t`) that the C source declares but never uses.
