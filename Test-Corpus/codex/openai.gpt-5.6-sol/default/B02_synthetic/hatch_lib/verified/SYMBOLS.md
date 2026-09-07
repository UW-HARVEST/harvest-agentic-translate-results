# Dynamic symbol surface

Mechanically extracted with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-kivdGu.so
nm -D --defined-only target/release/libhatch_lib.so
```

Only globally defined dynamic symbols (`T`) are part of the C library's public
surface. The C library's undefined symbols are the libc functions `difftime`,
`free`, `malloc`, `memmove`, `memset`, `snprintf`, and `time`, plus standard
toolchain weak symbols; they are not library exports.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `add_three` | `T` | `add_three` | [x] |
| `apply_operation` | `T` | `apply_operation` | [x] |
| `complex_calc` | `T` | `complex_calc` | [x] |
| `compute_with_dynamic_memory` | `T` | `compute_with_dynamic_memory` | [x] |
| `get_time_based_value` | `T` | `get_time_based_value` | [x] |
| `hatch` | `T` | `hatch` | [x] |
| `increment_counter` | `T` | `increment_counter` | [x] |
| `manipulate_records` | `T` | `manipulate_records` | [x] |
| `multiply_add` | `T` | `multiply_add` | [x] |
| `process_pointer_data` | `T` | `process_pointer_data` | [x] |
| `shift_array_data` | `T` | `shift_array_data` | [x] |
| `update_accumulator` | `T` | `update_accumulator` | [x] |

Missing C exports in Rust: **0**.

