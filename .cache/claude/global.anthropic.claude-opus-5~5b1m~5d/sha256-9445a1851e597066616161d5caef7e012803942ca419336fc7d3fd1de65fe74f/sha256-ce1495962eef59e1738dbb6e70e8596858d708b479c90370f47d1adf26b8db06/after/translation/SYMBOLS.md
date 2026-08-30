# SYMBOLS.md — exported-symbol parity

Reference C library: `c_src/build/libharvest-work-0arZgJ.so`
built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`
(no `CMAKE_BUILD_TYPE` ⇒ `-O0`, **`NDEBUG` is NOT defined ⇒ `assert()` is live**).

Rust library: `translation/target/release/libload_png_mem_lib.so` (`cdylib`).

## `nm -D --defined-only`

| # | symbol | C type | C size | in Rust `.so` | Rust type |
|---|--------|--------|--------|---------------|-----------|
| 1 | `load_png_mem`         | `T` FUNC   | –   | yes | `T` FUNC |
| 2 | `cp_inflate`           | `T` FUNC   | –   | yes | `T` FUNC |
| 3 | `cp_error_reason`      | `B` OBJECT | 8   | yes | `B` OBJECT |
| 4 | `cp_fixed_table`       | `D` OBJECT | 320 | yes | `D` OBJECT |
| 5 | `cp_permutation_order` | `D` OBJECT | 19  | yes | `D` OBJECT |
| 6 | `cp_len_extra_bits`    | `D` OBJECT | 31  | yes | `D` OBJECT |
| 7 | `cp_len_base`          | `D` OBJECT | 124 | yes | `D` OBJECT |
| 8 | `cp_dist_extra_bits`   | `D` OBJECT | 32  | yes | `D` OBJECT |
| 9 | `cp_dist_base`         | `D` OBJECT | 128 | yes | `D` OBJECT |

Missing from Rust: **none**. Extra in Rust: none.
No renaming macros exist in `c_src/include/lib.h`, so linker names == source names.

`static` C functions (`cp_make_pixel*`, `cp_would_overflow`, `cp_ptr`,
`cp_peak_bits`, `cp_consume_bits`, `cp_read_bits`, `cp_rev16`, `cp_build`,
`cp_stored`, `cp_fixed`, `cp_decode`, `cp_dynamic`, `cp_block`, `cp_paeth`,
`cp_make32`, `cp_chunk`, `cp_find`, `cp_unfilter`, `cp_convert`,
`cp_get_alpha_for_indexed_image`, `cp_depalette`,
`cp_get_chunk_byte_length`, `cp_out_size`) are not exported by either library
and are all translated as private Rust `fn`s.

Undefined (imported) symbols in the C `.so`: `__assert_fail`, `calloc`, `free`,
`malloc`, `memcmp`, `memcpy`, `memset` — all libc. The Rust `.so` imports the
same libc set (`abort` instead of `__assert_fail`, plus the Rust runtime's own
`_Unwind_*`/`__rust_*`-free set because `panic = "abort"`); it has 0 missing or
undefined non-libc symbols.

## Reference `.data` layout (matters for the C's out-of-range table reads)

`.data` is 0x2a0 = 672 bytes at 0x6060 and contains exactly the six tables, in
**source order**, each 32-byte aligned:

```
rel 0   cp_fixed_table        320 B
rel 320 cp_permutation_order   19 B  + 13 pad
rel 352 cp_len_extra_bits      31 B  +  1 pad
rel 384 cp_len_base           124 B  +  4 pad
rel 512 cp_dist_extra_bits     32 B
rel 544 cp_dist_base          128 B   (ends at rel 672 == .bss start)
```

`.bss` (0x6300, 16 B) = `completed.0` (8 B, rel 672..680) then `cp_error_reason`
(8 B, rel 680..688). The RW `LOAD` segment ends at 0x6310 and the mapping is
page-rounded to 0x7000, so rel 688..4000 reads as zero and rel ≥ 4000 faults.

`src/lib.rs` models this blob in `blob_byte()`; Rust/LLVM order statics
differently so the model is what makes the C's out-of-range
`cp_len_*`/`cp_dist_*` indexing agree.
