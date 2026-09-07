# Configuration surface

Rows are derived from all globally exported C entry points, the two mutable
global-state modes, the callback selected by `apply_operation`, both sides of
each `if`, and the loop/input shapes explicitly distinguished by the C source.
There are no Cargo features, C preprocessor feature flags, byte-order modes,
or executable targets.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `increment_counter` | Random signed values from a zero counter; observe through `complex_calc`. | [x] |
| C2 | `increment_counter` | Repeated random signed values with a nonzero accumulated counter. | [x] |
| C3 | `update_accumulator` | Random signed values from a zero accumulator; observe through `process_pointer_data`. | [x] |
| C4 | `update_accumulator` | Repeated random signed values with the `old * 2 + value` state transition. | [x] |
| C5 | `apply_operation`, `add_three` | Callback is the library's `add_three`; randomized scalar arguments. | [x] |
| C6 | `apply_operation`, `multiply_add` | Callback is the library's `multiply_add`; randomized scalar arguments. | [x] |
| C7 | `apply_operation`, `complex_calc` | Callback is the library's `complex_calc` with synchronized nonzero counter state. | [x] |
| C8 | `apply_operation` | Callback is an external test callback; randomized scalar arguments. | [x] |
| C9 | `add_three` | Direct low-level call with randomized signed arguments, including integer boundaries. | [x] |
| C10 | `multiply_add` | Direct low-level call with randomized signed arguments, including overflow-shaped values. | [x] |
| C11 | `complex_calc` | Direct low-level call with zero counter state. | [x] |
| C12 | `complex_calc` | Direct low-level call after randomized counter mutations. | [x] |
| C13 | `shift_array_data` | `size > 1`, `shift_by = 1`: active move with one vacated element. | [x] |
| C14 | `shift_array_data` | `size > 2`, `1 < shift_by < size`: active move with many vacated elements. | [x] |
| C15 | `shift_array_data` | Nonempty array and `shift_by = 0`: inactive guard. | [x] |
| C16 | `shift_array_data` | Nonempty array and `shift_by < 0`: inactive guard. | [x] |
| C17 | `shift_array_data` | `shift_by == size` or `shift_by > size`: inactive guard. | [x] |
| C18 | `shift_array_data` | Empty shape (`size = 0`) with a null pointer: inactive guard. | [x] |
| C19 | `process_pointer_data` | Valid pointer with zero accumulator and randomized multiplier/value. | [x] |
| C20 | `process_pointer_data` | Valid pointer after randomized accumulator mutations. | [x] |
| C21 | `compute_with_dynamic_memory` | `count < 0`: unsigned-size allocation request, zero loop iterations, return `0`. | [x] |
| C22 | `compute_with_dynamic_memory` | `count = 0`: zero-size allocation, zero loop iterations, return `0`. | [x] |
| C23 | `compute_with_dynamic_memory` | `count = 1`: one initialized and summed element. | [x] |
| C24 | `compute_with_dynamic_memory` | `count > 1`: many initialized and summed elements. | [x] |
| C25 | `get_time_based_value` | `seed = 0`. | [x] |
| C26 | `get_time_based_value` | Random positive seeds whose `seed * 3600` does not overflow. | [x] |
| C27 | `get_time_based_value` | Random negative seeds whose `seed * 3600` does not overflow. | [x] |
| C28 | `get_time_based_value` | Seeds at and beyond the signed-product overflow boundary. | [x] |
| C29 | `manipulate_records` | `0 < shift < num_records` with exactly one record remaining. | [x] |
| C30 | `manipulate_records` | `0 < shift < num_records` with many records remaining. | [x] |
| C31 | `manipulate_records` | `shift = 0`: no move; sum all records. | [x] |
| C32 | `manipulate_records` | `shift < 0`: no move; caller provides the extended `num_records - shift` readable shape. | [x] |
| C33 | `manipulate_records` | `shift = num_records`: no move and zero loop iterations. | [x] |
| C34 | `manipulate_records` | `shift > num_records`: no move and zero loop iterations. | [x] |
| C35 | `manipulate_records` | Empty shape (`num_records = 0`, `shift = 0`) with a null pointer. | [x] |
| C36 | `manipulate_records` | Negative `num_records` with a nonnegative shift: non-positive loop bound. | [x] |
| C37 | `hatch` | First composed call from zero global state with randomized parameters. | [x] |
| C38 | `hatch` | Repeated composed calls with synchronized persistent counter/accumulator state. | [x] |
