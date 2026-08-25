# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` section, optional dependencies, or implicit
dependency features. `c_src/CMakeLists.txt` has no options or preprocessor
configuration branches.

| # | Rust feature set | C configuration | [ ] |
|---|------------------|-----------------|-----|
| B01 | empty (`--no-default-features --features ''`) | default CMake configuration | [x] |

## Runtime Configurations

Rows are derived from all defined public C symbols and the state, callback,
length, and shift conditions used by `c_src/src/lib.c`. Arithmetic test values
stay within ranges where C signed arithmetic is defined.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C01 | `increment_counter` | Initial and accumulated counter state; randomized negative, zero, and positive `value`; ignored second argument varied | [x] |
| C02 | `update_accumulator` | Initial and accumulated accumulator state; randomized negative, zero, and positive `value`; ignored second argument varied | [x] |
| C03 | `add_three` | Randomized negative, zero, and positive scalar triples | [x] |
| C04 | `multiply_add` | Randomized scalar triples with products and sums in the defined `int` range | [x] |
| C05 | `complex_calc` | Initial zero counter with randomized scalar triples | [x] |
| C06 | `complex_calc` | Nonzero accumulated counter with randomized scalar triples | [x] |
| C07 | `apply_operation`, `add_three` | Callback points to the library's `add_three`; randomized scalar triples | [x] |
| C08 | `apply_operation`, `multiply_add` | Callback points to the library's `multiply_add`; randomized scalar triples | [x] |
| C09 | `apply_operation`, `complex_calc` | Callback points to the library's `complex_calc` with nonzero counter; randomized scalar triples | [x] |
| C10 | `shift_array_data` | Empty array shape: `size == 0`, `shift_by == 0` | [x] |
| C11 | `shift_array_data` | One-element array with inactive zero shift | [x] |
| C12 | `shift_array_data` | Many elements, active `shift_by == 1` | [x] |
| C13 | `shift_array_data` | Many elements, active interior shift `1 < shift_by < size - 1` | [x] |
| C14 | `shift_array_data` | Many elements, active boundary shift `shift_by == size - 1` | [x] |
| C15 | `shift_array_data` | Many elements, rejected nonpositive shift; array remains unchanged | [x] |
| C16 | `shift_array_data` | Many elements, rejected `shift_by >= size`; array remains unchanged | [x] |
| C17 | `process_pointer_data` | Pointer to one valid `int`, initial zero accumulator, randomized value and multiplier | [x] |
| C18 | `process_pointer_data` | Pointer to one valid `int`, nonzero accumulated accumulator, randomized value and multiplier | [x] |
| C19 | `compute_with_dynamic_memory` | `count == 0`; empty generated array | [x] |
| C20 | `compute_with_dynamic_memory` | `count == 1`; one generated element | [x] |
| C21 | `compute_with_dynamic_memory` | `count > 1`; many generated elements with randomized base and count | [x] |
| C22 | `get_time_based_value` | Randomized negative, zero, and positive seeds for which `seed * 3600` is in the defined `int` range | [x] |
| C23 | `manipulate_records` | Empty record set: `num_records == 0`, `shift == 0` | [x] |
| C24 | `manipulate_records` | One record, inactive `shift == 0` | [x] |
| C25 | `manipulate_records` | Many records, inactive `shift == 0`; sum every value | [x] |
| C26 | `manipulate_records` | Many records, active `shift == 1`; overlapping move and sum | [x] |
| C27 | `manipulate_records` | Many records, active interior shift `1 < shift < num_records - 1` | [x] |
| C28 | `manipulate_records` | Many records, active boundary shift `shift == num_records - 1` | [x] |
| C29 | `manipulate_records` | Rejected `shift >= num_records`; no access and zero total | [x] |
| C30 | `hatch` | Fresh global state; randomized four-parameter end-to-end operation | [x] |
| C31 | `hatch` | Repeated calls with accumulated global counter and accumulator state | [x] |
