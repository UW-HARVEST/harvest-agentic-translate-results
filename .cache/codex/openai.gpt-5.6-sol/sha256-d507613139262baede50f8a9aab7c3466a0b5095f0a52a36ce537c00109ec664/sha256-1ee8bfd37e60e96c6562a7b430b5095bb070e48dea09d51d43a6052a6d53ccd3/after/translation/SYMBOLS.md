# Dynamic Symbol Surface

Derived with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-rhgXPt.so
nm -D --defined-only target/release/libload_png_mem_lib.so
```

| C symbol | Kind | Rust export | Status |
|----------|------|-------------|--------|
| `cp_inflate` | function | `cp_inflate` | [x] |
| `load_png_mem` | function | `load_png_mem` | [x] |
| `cp_dist_base` | writable object (`uint32_t[32]`) | `cp_dist_base` | [x] |
| `cp_dist_extra_bits` | writable object (`uint8_t[32]`) | `cp_dist_extra_bits` | [x] |
| `cp_error_reason` | writable object (`const char *`) | `cp_error_reason` | [x] |
| `cp_fixed_table` | writable object (`uint8_t[320]`) | `cp_fixed_table` | [x] |
| `cp_len_base` | writable object (`uint32_t[31]`) | `cp_len_base` | [x] |
| `cp_len_extra_bits` | writable object (`uint8_t[31]`) | `cp_len_extra_bits` | [x] |
| `cp_permutation_order` | writable object (`uint8_t[19]`) | `cp_permutation_order` | [x] |

Missing C definitions in Rust: **0**.

The C library's undefined dynamic references are libc/toolchain functions only:
`__assert_fail`, `calloc`, `free`, `malloc`, `memcmp`, `memcpy`, and `memset`
(plus weak ELF runtime hooks).

Final verification after the no-feature release build found 9 definitions in
each library, an empty C-minus-Rust diff, an empty Rust-minus-C diff, and no
unresolved `cp_*`/`load_png_mem` references.
