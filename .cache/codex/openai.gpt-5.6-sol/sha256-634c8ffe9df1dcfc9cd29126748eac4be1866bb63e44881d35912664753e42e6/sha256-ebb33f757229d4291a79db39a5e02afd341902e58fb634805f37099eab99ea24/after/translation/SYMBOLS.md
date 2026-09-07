# Dynamic symbol surface

Derived from:

```text
nm -D --defined-only --format=posix ../c_src/build/libharvest-work-qzbIr9.so
nm -D --defined-only --format=posix target/release/liboverunder_lib.so
```

The public header declares only `overunder`, but the C shared object exposes
all five non-static function definitions. Therefore all five are part of the
observable shared-library ABI for parity purposes.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `copy_data_block` | `T` | `copy_data_block` | [x] |
| `handle_pointer_operations` | `T` | `handle_pointer_operations` | [x] |
| `overunder` | `T` | `overunder` | [x] |
| `process_with_fallthrough` | `T` | `process_with_fallthrough` | [x] |
| `safe_double_to_int` | `T` | `safe_double_to_int` | [x] |

Missing C symbols in Rust: **0**.

Undefined non-runtime symbols required from either library: **0**. Both
libraries intentionally depend on system/runtime symbols (for example libc,
libm, and the Rust runtime).
