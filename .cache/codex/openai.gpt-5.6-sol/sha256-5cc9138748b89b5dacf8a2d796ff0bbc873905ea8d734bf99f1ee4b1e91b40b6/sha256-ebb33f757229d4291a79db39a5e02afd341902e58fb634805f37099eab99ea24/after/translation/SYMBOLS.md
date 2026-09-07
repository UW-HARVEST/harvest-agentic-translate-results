# Dynamic symbol surface

Source library: `../c_src/build/libharvest-work-63lrKy.so`

Mechanical command:

```text
nm -D --defined-only ../c_src/build/libharvest-work-63lrKy.so
```

| # | C symbol | kind | Rust export | status |
|---|----------|------|-------------|--------|
| 1 | `convert_pix` | function (`T`) | `convert_pix` | [x] |
| 2 | `cp_dist_base` | data (`D`) | `cp_dist_base` | [x] |
| 3 | `cp_dist_extra_bits` | data (`D`) | `cp_dist_extra_bits` | [x] |
| 4 | `cp_error_reason` | BSS/data (`B`) | `cp_error_reason` | [x] |
| 5 | `cp_fixed_table` | data (`D`) | `cp_fixed_table` | [x] |
| 6 | `cp_inflate` | function (`T`) | `cp_inflate` | [x] |
| 7 | `cp_len_base` | data (`D`) | `cp_len_base` | [x] |
| 8 | `cp_len_extra_bits` | data (`D`) | `cp_len_extra_bits` | [x] |
| 9 | `cp_permutation_order` | data (`D`) | `cp_permutation_order` | [x] |

Missing C symbols in Rust: **0**.

Undefined non-libc C symbols in Rust: **0**.

