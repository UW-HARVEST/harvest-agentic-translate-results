# Exported symbol surface

Derived with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-6PFtVb.so
nm -D --defined-only target/release/libcheckshift_lib.so
```

| # | C symbol | Rust symbol | Status |
|---|----------|-------------|--------|
| 1 | `add_with_static` | `add_with_static` | present |
| 2 | `apply_operation` | `apply_operation` | present |
| 3 | `checkshift` | `checkshift` | present |
| 4 | `compute_checksum` | `compute_checksum` | present |
| 5 | `execute_operation` | `execute_operation` | present |
| 6 | `get_operation` | `get_operation` | present |
| 7 | `init_state` | `init_state` | present |
| 8 | `multiply_with_static` | `multiply_with_static` | present |
| 9 | `shift_with_static` | `shift_with_static` | present |
| 10 | `xor_operation` | `xor_operation` | present |

Missing C exports in Rust: **0**.

- [x] Final `nm -D --defined-only` diff is empty.
- [x] Both shared libraries contain 10 defined public symbols.
- [x] `ldd -r` reports no unresolved relocations.
