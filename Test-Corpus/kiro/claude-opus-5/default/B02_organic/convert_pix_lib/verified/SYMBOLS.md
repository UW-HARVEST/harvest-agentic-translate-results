# SYMBOLS.md — exported-symbol parity

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-h0yr0P.so
nm -D --defined-only translation/target/release/libconvert_pix_lib.so
```

`c_src/src/lib.c` is the only translation unit, and `c_src/include/lib.h`
declares only `cp_pixel_t` + `convert_pix`. Everything else that is exported is
a non-`static` file-scope definition in `lib.c`.

## Symbol table

| # | C symbol | nm type (C) | kind / C declaration | in Rust `.so`? | nm type (Rust) |
|---|----------|-------------|----------------------|----------------|----------------|
| 1 | `convert_pix`          | `T` | `void convert_pix(int bpp,int w,int h,uint8_t*,cp_pixel_t*)` | yes | `T` |
| 2 | `cp_inflate`           | `T` | `int cp_inflate(void*,int,void*,int)` (exported, not in header) | yes | `T` |
| 3 | `cp_error_reason`      | `B` | `const char *cp_error_reason;` (tentative def → .bss) | yes | `B` |
| 4 | `cp_fixed_table`       | `D` | `uint8_t cp_fixed_table[288+32]` | yes | `D` |
| 5 | `cp_permutation_order` | `D` | `uint8_t cp_permutation_order[19]` | yes | `D` |
| 6 | `cp_len_extra_bits`    | `D` | `uint8_t cp_len_extra_bits[29+2]` | yes | `D` |
| 7 | `cp_len_base`          | `D` | `uint32_t cp_len_base[29+2]` | yes | `D` |
| 8 | `cp_dist_extra_bits`   | `D` | `uint8_t cp_dist_extra_bits[30+2]` | yes | `D` |
| 9 | `cp_dist_base`         | `D` | `uint32_t cp_dist_base[30+2]` | yes | `D` |

**Missing from the Rust `.so`: none.** The symbol diff
(`comm -23 c_syms r_syms`) is empty. No stubs were added; every symbol above is
backed by a real translation of the corresponding C definition.

## `static` (non-exported) C functions — translated but not in `nm -D`

These are `static` in C, therefore *not* part of the ABI. They are all present
in `translation/src/lib.rs` and are exercised only indirectly.

| C static | reachable from an exported symbol? |
|----------|------------------------------------|
| `cp_make_pixel`, `cp_make_pixel_a` | yes — via `convert_pix` |
| `cp_would_overflow`, `cp_ptr`, `cp_peak_bits`, `cp_consume_bits`, `cp_read_bits`, `cp_rev16`, `cp_build`, `cp_stored`, `cp_fixed`, `cp_decode`, `cp_dynamic`, `cp_block` | yes — via `cp_inflate` |
| `cp_paeth`, `cp_make32`, `cp_chunk`, `cp_find`, `cp_unfilter` | **NO — dead code.** Nothing in `lib.c` calls them and they are `static`, so they are unreachable through the ABI and cannot be differentially tested. Their Rust counterparts are likewise `unsafe fn` (not exported), so ABI parity is unaffected. |

## Undefined-symbol check (Rust `.so`)

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind
imports (`malloc`, `free`, `memcpy`, `memmove`, `memset`, `bcmp`, `abort`,
`__errno_location`, `_Unwind_*`, `dl_iterate_phdr`, `pthread_key_*`, …).
**0 missing/undefined non-libc symbols.**

## Build-configuration note (affects Phases B, C and D)

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and no `NDEBUG`, so the C `.so`
built by the command in the task has `assert()` **live** — confirmed by
`__assert_fail@GLIBC_2.2.5` in its undefined symbols. A failing assert calls
`abort()`, so the Rust reproduces the assertions under the default `c_asserts`
feature (with `panic = "abort"` this aborts on exactly the same inputs).
`--no-default-features` drops them, matching a C build with `-DNDEBUG`.

Adding the asserts does not change the exported ABI: the symbol diff above is
empty for **both** feature combinations, and `tests/phase_d_symbols.rs` asserts
that the two sides agree about whether asserts are compiled in
(`d_assert_configuration_matches`) so a mismatched pairing cannot silently pass.

## Automated checks

`tests/phase_d_symbols.rs` re-derives this file at test time:

* `d_exported_symbol_parity` — `nm -D --defined-only` on both `.so`s; the C set
  must be a subset of the Rust set, the nine ABI names must be present in both,
  and the C must export nothing this file omits.
* `d_no_unresolved_non_libc_imports` — `nm -D --undefined-only` on the Rust `.so`
  contains only libc / unwinder / toolchain names (49 of them).
* `d_assert_configuration_matches` — see above.

`run_all.sh` additionally does a raw `diff` of the two sorted `nm -D` outputs for
each feature combination and fails if it is non-empty.
