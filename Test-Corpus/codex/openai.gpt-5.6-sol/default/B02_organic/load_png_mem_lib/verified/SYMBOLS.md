# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-O1k6j1.so
nm -D --defined-only target/release/libload_png_mem_lib.so
```

| # | C symbol | kind | Rust symbol | status |
|---|----------|------|-------------|--------|
| 1 | `cp_dist_base` | data | `cp_dist_base` | [x] |
| 2 | `cp_dist_extra_bits` | data | `cp_dist_extra_bits` | [x] |
| 3 | `cp_error_reason` | BSS data | `cp_error_reason` | [x] |
| 4 | `cp_fixed_table` | data | `cp_fixed_table` | [x] |
| 5 | `cp_inflate` | function | `cp_inflate` | [x] |
| 6 | `cp_len_base` | data | `cp_len_base` | [x] |
| 7 | `cp_len_extra_bits` | data | `cp_len_extra_bits` | [x] |
| 8 | `cp_permutation_order` | data | `cp_permutation_order` | [x] |
| 9 | `load_png_mem` | function | `load_png_mem` | [x] |

The C library's undefined dynamic references are only its libc/assert runtime
dependencies: `__assert_fail`, `calloc`, `free`, `malloc`, `memcmp`, `memcpy`,
and `memset` (plus weak toolchain symbols). There are no undefined project
symbols.

Missing C-defined symbols in Rust: **0**.
