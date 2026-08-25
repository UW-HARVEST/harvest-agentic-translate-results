# Dynamic Symbol Surface

Measured from the default C build with:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

The Rust comparison library is `target/debug/libbuffapp_lib.so`.

| # | C symbol | type | Rust export |
|---|----------|------|-------------|
| 1 | `append_to_buffer` | `T` | [x] |
| 2 | `buffapp` | `T` | [x] |
| 3 | `create_buffer` | `T` | [x] |
| 4 | `destroy_buffer` | `T` | [x] |
| 5 | `get_operation_name` | `T` | [x] |
| 6 | `perform_operation` | `T` | [x] |

Missing C-defined dynamic symbols in Rust: **0**.

