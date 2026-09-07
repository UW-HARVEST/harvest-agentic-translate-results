# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-e5zhlY.so
nm -D --defined-only target/release/libsiphash_lib.so
```

| C symbol | C type | Rust symbol | Rust type | Status |
|----------|--------|-------------|-----------|--------|
| `siphash` | `T` | `siphash` | `T` | [x] present |
| `stbds_hash_bytes` | `T` | `stbds_hash_bytes` | `T` | [x] present |

The C library's remaining dynamic symbols are undefined or weak runtime
imports: `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`, `printf`, and `puts`. They are supplied by
the platform runtime and are not library API exports.

Missing C API symbols in Rust: **0**.
