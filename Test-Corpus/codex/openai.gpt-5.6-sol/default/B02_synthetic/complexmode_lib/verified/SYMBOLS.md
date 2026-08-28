# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-l4N9HM.so
nm -D --defined-only target/release/libcomplexmode_lib.so
```

| C symbol | Rust symbol | Status |
|----------|-------------|--------|
| `check_permissions` | `check_permissions` | present |
| `compare_operations` | `compare_operations` | present |
| `complexmode` | `complexmode` | present |
| `copy_and_sum` | `copy_and_sum` | present |
| `create_result_string` | `create_result_string` | present |
| `multiply_with_log` | `multiply_with_log` | present |
| `safe_add` | `safe_add` | present |

The C shared object has no other globally defined dynamic symbols. Its
undefined symbols are libc/toolchain dependencies, not library API symbols.

- [x] Final symbol diff is empty.
- [x] Rust has no undefined non-libc library API symbols.
