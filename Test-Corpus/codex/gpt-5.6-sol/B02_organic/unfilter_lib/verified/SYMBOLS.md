# Dynamic symbol surface

Derived from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/release/libunfilter_lib.so
```

| C symbol | kind | Rust symbol | status |
|----------|------|-------------|--------|
| `cp_dist_base` | initialized data | `cp_dist_base` | present |
| `cp_dist_extra_bits` | initialized data | `cp_dist_extra_bits` | present |
| `cp_error_reason` | zero-initialized data | `cp_error_reason` | present |
| `cp_fixed_table` | initialized data | `cp_fixed_table` | present |
| `cp_inflate` | function | `cp_inflate` | present |
| `cp_len_base` | initialized data | `cp_len_base` | present |
| `cp_len_extra_bits` | initialized data | `cp_len_extra_bits` | present |
| `cp_permutation_order` | initialized data | `cp_permutation_order` | present |
| `unfilter` | function | `unfilter` | present |

Missing from Rust: **0**

Extra Rust exports: **0**

Undefined non-libc C-library symbols in Rust: **0**
