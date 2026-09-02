# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-2sETlw.so
nm -D --defined-only translation/target/release/libunfilter_lib.so
```

The C library is built from a single TU (`c_src/src/lib.c`); everything declared
`static` there has internal linkage and is therefore **not** part of the dynamic
symbol table. The exported surface is exactly the 9 symbols below.

## Defined dynamic symbols

| # | symbol | C type / signature | nm class (C) | in C `.so` | in Rust `.so` | nm class (Rust) |
|---|--------|--------------------|--------------|-----------|--------------|-----------------|
| 1 | `cp_dist_base`         | `uint32_t[30+2]`                                  | `D` | yes | yes | `D` |
| 2 | `cp_dist_extra_bits`   | `uint8_t[30+2]`                                   | `D` | yes | yes | `D` |
| 3 | `cp_error_reason`      | `const char *`                                    | `B` | yes | yes | `B` |
| 4 | `cp_fixed_table`       | `uint8_t[288+32]`                                 | `D` | yes | yes | `D` |
| 5 | `cp_inflate`           | `int (void*, int, void*, int)`                     | `T` | yes | yes | `T` |
| 6 | `cp_len_base`          | `uint32_t[29+2]`                                  | `D` | yes | yes | `D` |
| 7 | `cp_len_extra_bits`    | `uint8_t[29+2]`                                   | `D` | yes | yes | `D` |
| 8 | `cp_permutation_order` | `uint8_t[19]`                                     | `D` | yes | yes | `D` |
| 9 | `unfilter`             | `int (int, int, int, uint8_t*)`                    | `T` | yes | yes | `T` |

**Missing from Rust `.so`: none.** The symbol diff is empty; no wrapper had to be
added and no C module was left untranslated. `c_src` contains exactly one
translation unit (`src/lib.c`, 478 lines) plus a 2-line header, and every
function in it — including the `static` ones — has a counterpart in
`translation/src/lib.rs`.

## `static` (internal-linkage) C functions — not exported, but translated

These must **not** appear in `nm -D` for either library. They are listed to show
the translation is complete, not merely export-compatible.

| C `static` symbol | Rust counterpart | notes |
|---|---|---|
| `cp_make_pixel_a`    | `cp_make_pixel_a`    | dead code in C too |
| `cp_make_pixel`      | `cp_make_pixel`      | dead code in C too |
| `cp_would_overflow`  | `cp_would_overflow`  | only used inside an `assert` |
| `cp_ptr`             | `cp_ptr`             | |
| `cp_peak_bits`       | `cp_peak_bits`       | |
| `cp_consume_bits`    | `cp_consume_bits`    | |
| `cp_read_bits`       | `cp_read_bits`       | |
| `cp_rev16`           | `cp_rev16`           | |
| `cp_build`           | `cp_build`           | |
| `cp_stored`          | `cp_stored`          | |
| `cp_fixed`           | `cp_fixed`           | |
| `cp_decode`          | `cp_decode`          | |
| `cp_dynamic`         | `cp_dynamic`         | |
| `cp_block`           | `cp_block`           | |
| `cp_paeth`           | `cp_paeth`           | |
| `cp_make32`          | `cp_make32`          | |
| `cp_chunk`           | `cp_chunk`           | dead code in C too (no caller) |
| `cp_find`            | `cp_find`            | dead code in C too (no caller) |

`struct cp_pixel_t`, `struct cp_image_t`, `struct cp_state_t`, `struct
cp_raw_png_t` are all mirrored as `#[repr(C)]` in Rust. `cp_state_t`'s layout is
load-bearing: `cp_decode` can evaluate `tree[-1]`, so `lookup[511]` must
immediately precede `lit[0]`, `lit[287]` must precede `dst[0]`, and `dst[31]`
must precede `len[0]`. Verified by test `c32_state_layout_matches_c` in `tests/phase_b_inflate.rs`.

## Undefined symbols

`nm -D --undefined-only` on the Rust `.so` lists only libc/`libgcc_s` unwinder
imports (`memcpy`, `memset`, `malloc`, `calloc`, `free`, `abort`, `_Unwind_*`,
`__tls_get_addr`, …). **0 missing/undefined non-libc symbols.**

The C `.so` additionally imports `__assert_fail` because the CMake build sets no
`CMAKE_BUILD_TYPE` and therefore never defines `NDEBUG` — see the note at the top
of `ERRORS.md`.
