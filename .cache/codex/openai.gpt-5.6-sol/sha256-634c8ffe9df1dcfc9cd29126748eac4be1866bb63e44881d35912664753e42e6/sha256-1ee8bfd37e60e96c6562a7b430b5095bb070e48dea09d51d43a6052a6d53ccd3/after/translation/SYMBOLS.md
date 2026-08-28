# Dynamic Symbol Surface

Source command:

```text
nm -D --defined-only ../c_src/build/libharvest-work-zFkBcY.so
```

The C shared object has five defined public dynamic symbols. The Rust release
shared object was checked with:

```text
nm -D --defined-only target/release/liboverunder_lib.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `copy_data_block` | `T` | `copy_data_block` | [x] |
| `handle_pointer_operations` | `T` | `handle_pointer_operations` | [x] |
| `overunder` | `T` | `overunder` | [x] |
| `process_with_fallthrough` | `T` | `process_with_fallthrough` | [x] |
| `safe_double_to_int` | `T` | `safe_double_to_int` | [x] |

No C-defined symbol is missing from the Rust shared object. The final symmetric
symbol diff is empty, and `ldd -r target/release/liboverunder_lib.so` reports no
unresolved dynamic imports.
