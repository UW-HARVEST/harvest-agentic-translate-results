# Exported Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-caSExF.so
```

Only symbols defined by the library are listed; imported libc symbols are not
part of the library's public export surface.

| C symbol | Rust export |
|----------|-------------|
| `apply_operation` | [x] exact match |
| `charinbuf` | [x] exact match |
| `create_buffer` | [x] exact match |
| `decrement_counter` | [x] exact match |
| `find_char_in_buffer` | [x] exact match |
| `increment_counter` | [x] exact match |
| `is_string_empty` | [x] exact match |
| `multiply_counter` | [x] exact match |
| `reset_counter` | [x] exact match |
| `validate_uint16_range` | [x] exact match |

Missing from Rust: **0**

