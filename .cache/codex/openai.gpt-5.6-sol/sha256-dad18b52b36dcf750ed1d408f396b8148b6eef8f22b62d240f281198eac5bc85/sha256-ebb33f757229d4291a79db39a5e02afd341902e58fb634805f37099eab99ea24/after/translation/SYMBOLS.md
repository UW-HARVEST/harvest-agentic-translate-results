# Dynamic symbol surface

Mechanically derived from:

```text
nm -D --defined-only --format=posix ../c_src/build/libharvest-work-phwjKh.so
nm -D --defined-only --format=posix target/release/libmathop_lib.so
```

The C library exports 12 public functions. The Rust library exports all 12
under the same unmangled names.

| # | C symbol | Rust symbol present |
|---|----------|---------------------|
| 1 | `add_operation` | [x] |
| 2 | `allocate_results` | [x] |
| 3 | `divide_operation` | [x] |
| 4 | `get_computation_timestamp` | [x] |
| 5 | `get_operation_priority` | [x] |
| 6 | `is_valid_operation` | [x] |
| 7 | `mathop` | [x] |
| 8 | `modulo_operation` | [x] |
| 9 | `multiply_operation` | [x] |
| 10 | `perform_computation_with_history` | [x] |
| 11 | `select_operation` | [x] |
| 12 | `subtract_operation` | [x] |

Missing C symbols in Rust: **0**.
