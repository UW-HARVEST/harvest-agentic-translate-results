# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

The C shared object has 12 defined public dynamic symbols. The status column
records whether the exact name is also defined by
`target/debug/libhatch_lib.so`.

| # | C symbol | Type | Rust export |
|---|----------|------|-------------|
| 1 | `add_three` | `T` | [x] |
| 2 | `apply_operation` | `T` | [x] |
| 3 | `complex_calc` | `T` | [x] |
| 4 | `compute_with_dynamic_memory` | `T` | [x] |
| 5 | `get_time_based_value` | `T` | [x] |
| 6 | `hatch` | `T` | [x] |
| 7 | `increment_counter` | `T` | [x] |
| 8 | `manipulate_records` | `T` | [x] |
| 9 | `multiply_add` | `T` | [x] |
| 10 | `process_pointer_data` | `T` | [x] |
| 11 | `shift_array_data` | `T` | [x] |
| 12 | `update_accumulator` | `T` | [x] |

Missing C symbols in Rust: **0**.

Rust-only defined public symbols: **0**.

