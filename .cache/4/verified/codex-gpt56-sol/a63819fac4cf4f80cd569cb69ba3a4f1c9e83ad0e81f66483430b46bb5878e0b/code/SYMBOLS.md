# Dynamic Symbol Surface

Source library: `c_src/build/libtranslated_rust.so`

Command: `nm -D --defined-only c_src/build/libtranslated_rust.so`

| # | symbol | C type | Rust export |
|---|--------|--------|-------------|
| 1 | `cp_dist_base` | `D` | [x] |
| 2 | `cp_dist_extra_bits` | `D` | [x] |
| 3 | `cp_error_reason` | `B` | [x] |
| 4 | `cp_fixed_table` | `D` | [x] |
| 5 | `cp_inflate` | `T` | [x] |
| 6 | `cp_len_base` | `D` | [x] |
| 7 | `cp_len_extra_bits` | `D` | [x] |
| 8 | `cp_permutation_order` | `D` | [x] |
| 9 | `load_png_mem` | `T` | [x] |

The defined-symbol diff against
`target/release/libload_png_mem_lib.so` is empty. Both libraries' undefined
symbols are platform runtime/libc symbols; neither has an undefined
library-owned `cp_*` or `load_png_mem` symbol.
