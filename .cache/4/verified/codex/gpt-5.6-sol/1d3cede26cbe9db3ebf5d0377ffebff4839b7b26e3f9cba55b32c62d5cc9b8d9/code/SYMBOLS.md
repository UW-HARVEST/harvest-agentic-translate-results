# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only --format=posix c_src/build/libtranslated_rust.so
```

| C symbol | Type | Rust export | Status |
|----------|------|-------------|--------|
| `copy_data_block` | `T` | `copy_data_block` | [x] |
| `handle_pointer_operations` | `T` | `handle_pointer_operations` | [x] |
| `overunder` | `T` | `overunder` | [x] |
| `process_with_fallthrough` | `T` | `process_with_fallthrough` | [x] |
| `safe_double_to_int` | `T` | `safe_double_to_int` | [x] |

The C shared object also has dynamic imports for the libc/libm symbols
`memcpy`, `printf`, `putchar`, `sqrt`, and `strncpy`. These are dependencies,
not definitions owned by this library.

