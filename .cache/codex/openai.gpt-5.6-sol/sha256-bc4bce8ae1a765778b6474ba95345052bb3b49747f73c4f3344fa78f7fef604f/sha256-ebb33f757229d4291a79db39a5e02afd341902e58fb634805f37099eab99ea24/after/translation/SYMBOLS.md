# Exported symbol parity

Source command:

```text
nm -D --defined-only ../c_src/build/libharvest-work-kvAlTr.so
```

| C symbol | Kind | Rust export | Status |
|----------|------|-------------|--------|
| `pinflate` | function (`T`) | `pinflate` | present |
| `cp_fixed_table` | initialized data (`D`) | `cp_fixed_table` | present |
| `cp_permutation_order` | initialized data (`D`) | `cp_permutation_order` | present |
| `cp_len_extra_bits` | initialized data (`D`) | `cp_len_extra_bits` | present |
| `cp_len_base` | initialized data (`D`) | `cp_len_base` | present |
| `cp_dist_extra_bits` | initialized data (`D`) | `cp_dist_extra_bits` | present |
| `cp_dist_base` | initialized data (`D`) | `cp_dist_base` | present |
| `cp_error_reason` | zero-initialized data (`B`) | `cp_error_reason` | present |

Missing C symbols in Rust: **0**.

The C shared object has no other defined dynamic symbols. Its undefined symbols
are C-runtime imports (`calloc`, `free`, `memcpy`, `memset`, and
`__assert_fail`), not library API symbols.
