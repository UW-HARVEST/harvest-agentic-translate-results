# Dynamic symbol surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-aRikqo.so
nm -D --defined-only target/release/libpinflate_lib.so
```

The C shared object has eight globally defined dynamic symbols. The Rust
shared object exports all eight with the same names and object sizes.

| # | C symbol | Kind | Size (bytes) | Rust export |
|---|----------|------|--------------|-------------|
| 1 | `cp_dist_base` | object | 128 | present |
| 2 | `cp_dist_extra_bits` | object | 32 | present |
| 3 | `cp_error_reason` | object | 8 | present |
| 4 | `cp_fixed_table` | object | 320 | present |
| 5 | `cp_len_base` | object | 124 | present |
| 6 | `cp_len_extra_bits` | object | 31 | present |
| 7 | `cp_permutation_order` | object | 19 | present |
| 8 | `pinflate` | function | 667 in the C build | present |

The C object's undefined dynamic symbols are `calloc`, `free`, `memcpy`,
`memset`, and `__assert_fail`, plus the weak toolchain symbols
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, and `__gmon_start__`. These are libc/toolchain imports,
not library API. There are no undefined non-libc library symbols.

**Missing C-defined symbols in Rust: 0.**
