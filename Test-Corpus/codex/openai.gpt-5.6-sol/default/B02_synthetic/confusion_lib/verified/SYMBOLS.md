# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-wWTQN6.so
nm -D --defined-only target/release/libconfusion_lib.so
```

Only defined dynamic symbols are API candidates. The C header declares only
`confusion`, but the shared object also exposes the five non-static helper
functions below, so all six are part of the observed FFI surface.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `confuse_types` | `T` | `confuse_types` | [x] present |
| `confusion` | `T` | `confusion` | [x] present |
| `create_state` | `T` | `create_state` | [x] present |
| `destroy_state` | `T` | `destroy_state` | [x] present |
| `process_buffer` | `T` | `process_buffer` | [x] present |
| `update_flags` | `T` | `update_flags` | [x] present |

Missing C exports in Rust: **0**.

Undefined non-libc C API symbols in Rust: **0**.
