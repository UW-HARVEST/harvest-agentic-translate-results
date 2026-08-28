# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-hklH5o.so
```

| # | C symbol | C type | Rust export |
|---|----------|--------|-------------|
| 1 | `increment_counter` | `T` | present |
| 2 | `decrement_counter` | `T` | present |
| 3 | `multiply_counter` | `T` | present |
| 4 | `reset_counter` | `T` | present |
| 5 | `is_string_empty` | `T` | present |
| 6 | `find_char_in_buffer` | `T` | present |
| 7 | `create_buffer` | `T` | present |
| 8 | `validate_uint16_range` | `T` | present |
| 9 | `apply_operation` | `T` | present |
| 10 | `charinbuf` | `T` | present |

Missing from Rust: **0**

The remaining undefined symbols in each shared object are libc, compiler
runtime, or loader/runtime imports rather than library API symbols.
