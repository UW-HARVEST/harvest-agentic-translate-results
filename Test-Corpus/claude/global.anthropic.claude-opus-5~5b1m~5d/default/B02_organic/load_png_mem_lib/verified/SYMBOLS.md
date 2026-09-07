# SYMBOLS.md — exported-symbol parity

Reference C library built exactly as instructed:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-YXyZDY.so   (no CMAKE_BUILD_TYPE => -O0, asserts ON)
```

Rust library: `translation/target/release/libload_png_mem_lib.so`
(`cargo build --release`).

## `nm -D --defined-only` comparison

| # | symbol | C type/size | Rust type/size | present in Rust |
|---|--------|-------------|----------------|-----------------|
| 1 | `cp_inflate`           | `T` (FUNC)          | `T` (FUNC)          | yes |
| 2 | `load_png_mem`         | `T` (FUNC)          | `T` (FUNC)          | yes |
| 3 | `cp_fixed_table`       | `D` OBJECT 0x140=320| `D` OBJECT 320      | yes |
| 4 | `cp_permutation_order` | `D` OBJECT 0x13=19  | `D` OBJECT 19       | yes |
| 5 | `cp_len_extra_bits`    | `D` OBJECT 0x1f=31  | `D` OBJECT 31       | yes |
| 6 | `cp_len_base`          | `D` OBJECT 0x7c=124 | `D` OBJECT 124      | yes |
| 7 | `cp_dist_extra_bits`   | `D` OBJECT 0x20=32  | `D` OBJECT 32       | yes |
| 8 | `cp_dist_base`         | `D` OBJECT 0x80=128 | `D` OBJECT 128      | yes |
| 9 | `cp_error_reason`      | `B` OBJECT 8        | `B` OBJECT 8        | yes |

**Missing from Rust: none.** No module of `c_src` was left untranslated —
`c_src` consists of a single translation unit (`src/lib.c`, 758 lines) and every
one of its functions has a counterpart in `translation/src/lib.rs`
(`cp_make_pixel_a`, `cp_make_pixel`, `cp_would_overflow`, `cp_ptr`,
`cp_peak_bits`, `cp_consume_bits`, `cp_read_bits`, `cp_rev16`, `cp_build`,
`cp_stored`, `cp_fixed`, `cp_decode`, `cp_dynamic`, `cp_block`, `cp_inflate`,
`cp_paeth`, `cp_make32`, `cp_chunk`, `cp_find`, `cp_unfilter`, `cp_convert`,
`cp_get_alpha_for_indexed_image`, `cp_depalette`, `cp_get_chunk_byte_length`,
`cp_out_size`, `load_png_mem`).  The `static` C functions are not exported by
either library, so they are correctly not in the table above.

There are no namespace/renaming macros in `c_src/include/lib.h`, so no
macro-generated symbol names exist.

## Undefined (imported) symbols

The Rust `.so` imports only libc symbols, all of which the C `.so` also imports:

```
malloc, calloc, free, memcpy, memset, memcmp   (+ Rust runtime: none required
                                                 because panic = "abort")
```

Verified with `nm -D --undefined-only`; there are 0 non-libc undefined symbols.

## `.data` layout of the six exported tables — IMPORTANT

`cp_block` indexes `cp_len_extra_bits` / `cp_len_base` with `symbol - 257` and
`cp_dist_extra_bits` / `cp_dist_base` with a decoded distance symbol.  A corrupt
Huffman tree makes `cp_decode` return values up to 4095, so the C reads *past*
the end of the table and into whatever the linker put next.  In the reference
build the six tables are the whole of `.data` (0x6060, size 0x2a0 = 672 bytes):

| blob offset | object | bytes | pad |
|---|---|---|---|
| 0   | `cp_fixed_table`       | 320 | 0  |
| 320 | `cp_permutation_order` | 19  | 13 |
| 352 | `cp_len_extra_bits`    | 31  | 1  |
| 384 | `cp_len_base`          | 124 | 4  |
| 512 | `cp_dist_extra_bits`   | 32  | 0  |
| 544 | `cp_dist_base`         | 128 | 0  |
| 672 | (`.bss`: 8 pad bytes, then `cp_error_reason`) | | |

This is **source order**, which is what gcc emits at `-O0` (the reference build).
At `-O1`/`-O2`/`-O3`/`-Os` gcc emits the *reverse* order — measured:

```
-O0 : fixed_table, permutation_order, len_extra_bits, len_base, dist_extra_bits, dist_base
-O1+: dist_base, dist_extra_bits, len_base, len_extra_bits, permutation_order, fixed_table
```

`translation/src/lib.rs` models the `-O0` (reference) order in `blob_byte()`.
This was **fixed during verification** — the model previously encoded the `-O1+`
order, which made every out-of-range table read disagree with the reference
`.so` (e.g. `cp_len_extra_bits[32]`: reference reads `cp_len_base[0]`'s LSB = 3,
the old model returned `cp_permutation_order[0]` = 16).

## Result

`verify.sh` step 3 (`comm -23` of the two sorted `nm -D --defined-only` lists):

```
0 symbols missing from the Rust .so (9 exported by C)
```

Automated equivalents live in `tests/phase_d_symbols.rs`:

* `every_c_symbol_is_exported_by_rust` — set equality plus matching symbol
  *types*, and matching *sizes* for every OBJECT (function code sizes differ, as
  expected: `cp_inflate` is 667 bytes of `-O0` C and 3379 bytes of optimised
  Rust).
* `expected_symbol_set` — the pinned 9-symbol list above.
* `rust_so_has_no_unresolved_symbols` — `dlopen(RTLD_NOW)` on the Rust `.so`
  (which forces the loader to resolve every import) and a check that every libc
  symbol the C imports is also imported by Rust.  The single deliberate
  exception is `__assert_fail`: the reference C build has `assert()` enabled, the
  translation is `NDEBUG`-equivalent (see `ERRORS.md`, rows A1..A10).
  `memcmp` is satisfied by `bcmp` (the same function, LLVM's preferred name).
* `c_data_layout_is_source_order` — asserts the six tables sit at blob offsets
  0 / 320 / 352 / 384 / 512 / 544 in the reference `.so`, i.e. the premise the
  `blob_byte` model in `src/lib.rs` is built on.  If a future C build reorders
  `.data`, this test fails and names the model that has to change.
* `out_of_range_table_read_agrees` — a differential check that an out-of-range
  table read (literal/length symbols 286 and 287) produces identical results;
  `phase_c_errors::row48_out_of_range_table_reads` is the exhaustive version.
