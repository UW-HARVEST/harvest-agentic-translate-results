# Dynamic Symbol Surface

Source command:

```text
nm -D --defined-only c_src/build/libharvest-work-pddoiK.so
```

Only globally defined dynamic symbols (`T`) are part of the C library surface.
The Rust comparison was made against `target/release/libbuffapp_lib.so`.

| # | C symbol | C type | Rust export | Status |
|---|----------|--------|-------------|--------|
| 1 | `append_to_buffer` | `T` | `append_to_buffer` | present |
| 2 | `buffapp` | `T` | `buffapp` | present |
| 3 | `create_buffer` | `T` | `create_buffer` | present |
| 4 | `destroy_buffer` | `T` | `destroy_buffer` | present |
| 5 | `get_operation_name` | `T` | `get_operation_name` | present |
| 6 | `perform_operation` | `T` | `perform_operation` | present |

Missing C symbols in Rust: **0**.

There are no macro-generated public symbols and no non-library executable
target. Undefined entries in both shared objects are runtime/libc imports, not
missing library implementations.
