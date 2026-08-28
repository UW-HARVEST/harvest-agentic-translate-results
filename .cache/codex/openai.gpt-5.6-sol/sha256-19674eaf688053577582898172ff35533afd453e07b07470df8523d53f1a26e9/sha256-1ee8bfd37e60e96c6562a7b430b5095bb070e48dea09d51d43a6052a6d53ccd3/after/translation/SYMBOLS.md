# Dynamic Symbol Surface

Derived with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-m1bNJW.so
nm -D --defined-only target/release/libconfusion_lib.so
```

Only globally defined dynamic symbols reported by the C library are part of
this table. All six are functions defined in `c_src/src/lib.c`.

| C symbol | Rust export | Status |
|----------|-------------|--------|
| `confuse_types` | `confuse_types` | present |
| `confusion` | `confusion` | present |
| `create_state` | `create_state` | present |
| `destroy_state` | `destroy_state` | present |
| `process_buffer` | `process_buffer` | present |
| `update_flags` | `update_flags` | present |

Missing C symbols in Rust: **0**

- [x] Final release-build symbol diff is empty.
- [x] Rust has no undefined non-libc symbols required from the C library.
