# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-KQ5axu.so
nm -D --defined-only target/release/libcleanup_lib.so
```

Only globally defined C-library symbols are API symbols. Undefined GLIBC
imports and weak toolchain hooks are runtime dependencies, not public API.

| C symbol | C type | Rust type | Rust status |
|----------|--------|-----------|-------------|
| `cleanup` | `T` | `T` | present |
| `cleanup_resources` | `T` | `T` | present |
| `print_result` | `T` | `T` | present |

Missing C symbols in Rust: **0**

- [x] All C-defined dynamic API symbols are defined by the Rust shared object.
- [x] The Rust shared object has no unresolved dynamic symbols.
