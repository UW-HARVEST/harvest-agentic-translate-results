# Dynamic Symbol Surface

Derived with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-00V7NW.so
nm -D --defined-only target/release/libget_predict_func_lib.so
```

| C symbol | C type | Rust symbol present | Rust type |
|----------|--------|---------------------|-----------|
| `get_predict_func` | `T` | yes | `T` |

Missing C symbols in Rust: **0**

