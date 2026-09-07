# Dynamic Symbol Surface

Source of truth:

```text
nm -D --defined-only ../c_src/build/libharvest-work-smgN5h.so
```

The C library has seven defined public dynamic symbols. The release Rust
library was checked with:

```text
nm -D --defined-only target/release/libcomplexmode_lib.so
```

| C symbol | C type | Rust export |
|----------|--------|-------------|
| `check_permissions` | `T` | [x] |
| `compare_operations` | `T` | [x] |
| `complexmode` | `T` | [x] |
| `copy_and_sum` | `T` | [x] |
| `create_result_string` | `T` | [x] |
| `multiply_with_log` | `T` | [x] |
| `safe_add` | `T` | [x] |

Missing C exports in Rust: **0**

Undefined non-runtime/non-libc symbols in Rust: **0**

