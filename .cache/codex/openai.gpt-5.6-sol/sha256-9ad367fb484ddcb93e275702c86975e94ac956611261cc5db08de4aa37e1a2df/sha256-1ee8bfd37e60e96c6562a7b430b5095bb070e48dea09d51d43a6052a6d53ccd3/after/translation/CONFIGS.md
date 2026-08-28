# Configuration Surface

Mechanical axes derived from `c_src/src/lib.c` and `c_src/include/lib.h`:

- No Cargo features, C preprocessor modes, runtime mode flags, enums, formats,
  element types, or byte-order options exist.
- Mutable process state has two axes: `global_counter` and
  `global_accumulator`.
- `apply_operation` accepts a callback; the C pipeline uses `add_three`,
  `multiply_add`, and `complex_calc`.
- `shift_array_data` branches on `shift_by > 0 && shift_by < size`, then gives
  `memmove` and `memset` zero, one, or many elements.
- Dynamic-memory and record loops distinguish zero, one, and many iterations.
- `manipulate_records` branches on `shift > 0 && shift < num_records`.
- `hatch` composes every lower-level operation and preserves global state
  between calls.

The only build configuration is the featureless crate. Default and
`--no-default-features` select the same code.

| Feature selection | Phase B | Phase C |
|-------------------|---------|---------|
| default | [x] | [x] |
| `--no-default-features` | [x] | [x] |

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `increment_counter` | Zero, positive, and negative values from zero and previously nonzero counter state. | [x] |
| 2 | `update_accumulator` | Zero, positive, and negative values from zero and previously nonzero accumulator state; repeated updates exercise `old * 2 + value`. | [x] |
| 3 | `add_three` | Random triples spanning negative, zero, and positive integers. | [x] |
| 4 | `multiply_add` | Random triples spanning negative, zero, and positive integers. | [x] |
| 5 | `complex_calc` | Random triples with zero and nonzero counter state. | [x] |
| 6 | `apply_operation`, `add_three` | Callback is `add_three`; random triples. | [x] |
| 7 | `apply_operation`, `multiply_add` | Callback is `multiply_add`; random triples. | [x] |
| 8 | `apply_operation`, `complex_calc` | Callback is `complex_calc`; random triples with nonzero counter state. | [x] |
| 9 | `shift_array_data` | Active branch; one retained element and one zero-filled element (`size = 2`, `shift_by = 1`). | [x] |
| 10 | `shift_array_data` | Active branch; many retained elements and one zero-filled element (`size > 2`, `shift_by = 1`). | [x] |
| 11 | `shift_array_data` | Active branch; one retained element and many zero-filled elements (`shift_by = size - 1`). | [x] |
| 12 | `shift_array_data` | Active branch; many retained and many zero-filled elements (`1 < shift_by < size - 1`). | [x] |
| 13 | `shift_array_data` | Inactive branch at/below lower boundary: `shift_by = 0` and `shift_by < 0`; empty, one, and many-element buffers. | [x] |
| 14 | `shift_array_data` | Inactive branch at/above upper boundary: `shift_by = size` and `shift_by > size`; empty, one, and many-element buffers. | [x] |
| 15 | `process_pointer_data` | Random pointed-to values and multipliers with zero and nonzero accumulator state. | [x] |
| 16 | `compute_with_dynamic_memory` | `count = 0`; both loops execute zero times. | [x] |
| 17 | `compute_with_dynamic_memory` | `count = 1`; both loops execute once. | [x] |
| 18 | `compute_with_dynamic_memory` | Small positive `count > 1`; both loops execute many times. | [x] |
| 19 | `compute_with_dynamic_memory` | Negative `count`; allocation request uses converted size and both loops execute zero times. | [x] |
| 20 | `get_time_based_value` | Negative, zero, and positive seeds within and at the signed `seed * 3600` boundary. | [x] |
| 21 | `manipulate_records` | Active branch with one remaining record (`0 < shift < num_records`, `num_records - shift = 1`). | [x] |
| 22 | `manipulate_records` | Active branch with many remaining records (`0 < shift < num_records`, `num_records - shift > 1`). | [x] |
| 23 | `manipulate_records` | Inactive lower boundary (`shift = 0`) with one and many sum iterations. | [x] |
| 24 | `manipulate_records` | Inactive lower range (`shift < 0`); backing storage includes the extra records read by `num_records - shift`. | [x] |
| 25 | `manipulate_records` | Empty shape (`num_records = 0`, `shift = 0`); no move and zero sum iterations. | [x] |
| 26 | `manipulate_records` | Inactive upper boundary/range (`shift = num_records` and `shift > num_records`); zero sum iterations. | [x] |
| 27 | `hatch` | First call from freshly loaded zero global state; randomized parameter tuples. | [x] |
| 28 | `hatch` | Repeated calls with accumulated counter and accumulator state; randomized parameter tuples. | [x] |
