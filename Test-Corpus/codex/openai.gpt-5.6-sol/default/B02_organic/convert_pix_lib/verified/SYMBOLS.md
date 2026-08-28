# Dynamic symbol surface

Source library: `../c_src/build/libharvest-work-lPsfCn.so`

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-lPsfCn.so
```

| C symbol | ELF kind | Rust export | Status |
|----------|----------|-------------|--------|
| `cp_inflate` | `T` | `cp_inflate` | [x] |
| `convert_pix` | `T` | `convert_pix` | [x] |
| `cp_fixed_table` | `D` | `cp_fixed_table` | [x] |
| `cp_permutation_order` | `D` | `cp_permutation_order` | [x] |
| `cp_len_extra_bits` | `D` | `cp_len_extra_bits` | [x] |
| `cp_len_base` | `D` | `cp_len_base` | [x] |
| `cp_dist_extra_bits` | `D` | `cp_dist_extra_bits` | [x] |
| `cp_dist_base` | `D` | `cp_dist_base` | [x] |
| `cp_error_reason` | `B` | `cp_error_reason` | [x] |

The C object also imports the following runtime symbols. They are not library
exports and therefore are not required to be defined by the Rust object:
`__assert_fail`, `calloc`, `free`, `memcmp`, `memcpy`, and `memset`, plus the
usual weak ELF transaction/finalization hooks.

Defined-symbol diff: empty.
