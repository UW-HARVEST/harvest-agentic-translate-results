# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only --format=posix ../c_src/build/libharvest-work-ohM76p.so
nm -D --defined-only --format=posix target/release/libunfilter_lib.so
```

| C symbol | ELF type | C size (hex) | Rust export | Status |
|----------|----------|--------------|-------------|--------|
| `cp_dist_base` | `D` | `0x80` | `cp_dist_base` | present |
| `cp_dist_extra_bits` | `D` | `0x20` | `cp_dist_extra_bits` | present |
| `cp_error_reason` | `B` | `0x8` | `cp_error_reason` | present |
| `cp_fixed_table` | `D` | `0x140` | `cp_fixed_table` | present |
| `cp_inflate` | `T` | `0x29b` | `cp_inflate` | present |
| `cp_len_base` | `D` | `0x7c` | `cp_len_base` | present |
| `cp_len_extra_bits` | `D` | `0x1f` | `cp_len_extra_bits` | present |
| `cp_permutation_order` | `D` | `0x13` | `cp_permutation_order` | present |
| `unfilter` | `T` | `0x470` | `unfilter` | present |

Missing C symbols in Rust: **0**

