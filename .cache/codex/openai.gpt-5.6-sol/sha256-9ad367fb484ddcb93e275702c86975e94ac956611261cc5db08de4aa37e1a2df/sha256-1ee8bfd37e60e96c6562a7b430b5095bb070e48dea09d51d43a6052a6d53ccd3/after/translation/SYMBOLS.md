# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-I5OVhX.so
nm -D --defined-only target/release/libhatch_lib.so
```

The C shared object has 12 globally defined dynamic symbols. The Rust shared
object exports every one with the exact same name.

| # | C symbol | C type | Rust export |
|---|----------|--------|-------------|
| 1 | `increment_counter` | `T` | [x] |
| 2 | `update_accumulator` | `T` | [x] |
| 3 | `apply_operation` | `T` | [x] |
| 4 | `add_three` | `T` | [x] |
| 5 | `multiply_add` | `T` | [x] |
| 6 | `complex_calc` | `T` | [x] |
| 7 | `shift_array_data` | `T` | [x] |
| 8 | `process_pointer_data` | `T` | [x] |
| 9 | `compute_with_dynamic_memory` | `T` | [x] |
| 10 | `get_time_based_value` | `T` | [x] |
| 11 | `manipulate_records` | `T` | [x] |
| 12 | `hatch` | `T` | [x] |

Missing C exports in Rust: **0**

The C object's undefined dynamic symbols are libc/toolchain imports:
`difftime`, `free`, `malloc`, `memmove`, `memset`, `snprintf`, `time`,
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
and `__gmon_start__`. They are not library exports.
