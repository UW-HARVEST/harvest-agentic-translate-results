# Dynamic symbol surface

Source library: `../c_src/build/libharvest-work-jrm6bV.so`

Inventory command:

```text
nm -D --defined-only ../c_src/build/libharvest-work-jrm6bV.so
```

Only defined dynamic symbols are public library exports; undefined libc/runtime
imports are not library API symbols.

| C symbol | C type | Rust export |
|----------|--------|-------------|
| `add_operation` | `T` | [x] |
| `arrayfunc` | `T` | [x] |
| `compare_results_in_array` | `T` | [x] |
| `compute_scaled_value` | `T` | [x] |
| `compute_weighted_sum` | `T` | [x] |
| `init_result_array` | `T` | [x] |
| `modulo_operation` | `T` | [x] |
| `multiply_operation` | `T` | [x] |
| `process_with_foreach` | `T` | [x] |
| `safe_double_to_int` | `T` | [x] |
| `subtract_operation` | `T` | [x] |

Missing from Rust: **0**
