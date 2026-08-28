# Dynamic Symbol Surface

Reference library:
`../c_src/build/libharvest-work-giENQi.so`

Derived with:

```sh
nm -D --defined-only --format=posix \
  ../c_src/build/libharvest-work-giENQi.so
```

| C symbol | Kind | Rust export | Status |
|----------|------|-------------|--------|
| `add_operation` | `T` | `add_operation` | present |
| `arrayfunc` | `T` | `arrayfunc` | present |
| `compare_results_in_array` | `T` | `compare_results_in_array` | present |
| `compute_scaled_value` | `T` | `compute_scaled_value` | present |
| `compute_weighted_sum` | `T` | `compute_weighted_sum` | present |
| `init_result_array` | `T` | `init_result_array` | present |
| `modulo_operation` | `T` | `modulo_operation` | present |
| `multiply_operation` | `T` | `multiply_operation` | present |
| `process_with_foreach` | `T` | `process_with_foreach` | present |
| `safe_double_to_int` | `T` | `safe_double_to_int` | present |
| `subtract_operation` | `T` | `subtract_operation` | present |

Missing C symbols in Rust: **0**

Extra Rust project symbols: **0**

The C dynamic table also has weak undefined runtime symbols
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, and `__gmon_start__`. These are toolchain support imports,
not library API symbols.
